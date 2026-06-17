#[path = "processing/job_loop.rs"]
mod job_loop;
#[path = "processing/records.rs"]
mod records;
#[path = "processing/render_step.rs"]
mod render_step;
#[path = "processing/submit.rs"]
mod submit;

use std::sync::Arc;

use super::super::{
    models::{ProcessJobError, JOB_STATUS_RENDERING, JOB_STATUS_SUBMITTING},
    AgentState,
};
use crate::printer::PrintOptions;
use crate::storage::{
    save_render_artifact_result, transition_job_status, RenderArtifactJobUpdateInput,
};

pub(super) use job_loop::worker_loop;
pub(super) use records::load_job_for_processing;

pub(super) async fn process_job(
    state: Arc<AgentState>,
    job_id: &str,
) -> Result<(), ProcessJobError> {
    let job = load_job_for_processing(state.db_path.as_ref(), job_id)?;
    let print_options = parse_print_options(&job.print_options_json)?;
    let render_result = render_step::render_job_artifact(&state, &job, &print_options).await?;

    save_render_artifact_result(
        state.db_path.as_ref(),
        job_id,
        RenderArtifactJobUpdateInput {
            artifact_path: render_result.artifact_path.as_str(),
            output_kind: render_result.output_kind.as_str(),
            page_count: i64::from(render_result.page_count),
            page_width_pt: render_result.page_width_pt,
            page_height_pt: render_result.page_height_pt,
        },
    )
    .map_err(|err| {
        ProcessJobError::new(
            "DB_WRITE_FAILED",
            format!("failed to save render artifact metadata: {err}"),
        )
    })?;

    transition_rendering_to_submitting(&state, job_id)?;
    submit::submit_rendered_job(
        &state,
        &job,
        job_id,
        render_result.artifact_path.as_str(),
        print_options,
    )
    .await
}

fn parse_print_options(raw: &str) -> Result<PrintOptions, ProcessJobError> {
    serde_json::from_str(raw).map_err(|err| {
        ProcessJobError::new(
            "INVALID_PRINT_OPTIONS",
            format!("unable to parse print_options_json: {err}"),
        )
    })
}

fn transition_rendering_to_submitting(
    state: &AgentState,
    job_id: &str,
) -> Result<(), ProcessJobError> {
    let transitioned = transition_job_status(
        state.db_path.as_ref(),
        job_id,
        JOB_STATUS_RENDERING,
        JOB_STATUS_SUBMITTING,
        "render step completed, begin backend submission",
        None,
        None,
    )
    .map_err(|err| {
        ProcessJobError::new(
            "STATE_TRANSITION_FAILED",
            format!("failed transition rendering->submitting: {err}"),
        )
    })?;

    if !transitioned {
        return Err(ProcessJobError::new(
            "STATE_STALE",
            "job state changed before entering submitting",
        ));
    }

    Ok(())
}
