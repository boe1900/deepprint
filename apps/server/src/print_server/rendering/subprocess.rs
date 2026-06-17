use std::time::Duration;

use serde_json::Value;

use super::super::{models::ProcessJobError, JobRecord, PrintOptions};
use super::errors::map_render_error;
use crate::renderer::{self, RenderRequest, RenderResult};

pub(crate) async fn render_job_artifact_via_subprocess(
    job: &JobRecord,
    print_options: &PrintOptions,
    render_timeout_sec: u64,
) -> Result<RenderResult, ProcessJobError> {
    let data: Value = serde_json::from_str(&job.data_json).map_err(|err| {
        ProcessJobError::new(
            "INVALID_DATA_JSON",
            format!("unable to parse job data_json: {err}"),
        )
    })?;

    let render_request = RenderRequest {
        job_id: job.id.clone(),
        request_id: job.request_id.clone(),
        template_content: job.template_content.clone(),
        data,
        print_options: serde_json::to_value(print_options).map_err(|err| {
            ProcessJobError::new(
                "INVALID_PRINT_OPTIONS",
                format!("unable to serialize print options: {err}"),
            )
        })?,
    };

    let timeout = Duration::from_secs(render_timeout_sec.max(5));
    renderer::render_via_subprocess(&render_request, timeout)
        .await
        .map_err(map_render_error)
}
