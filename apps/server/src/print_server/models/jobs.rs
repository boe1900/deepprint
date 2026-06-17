#[derive(Debug)]
pub(crate) struct ProcessJobError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

impl ProcessJobError {
    pub(crate) fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
        }
    }

    pub(crate) fn retryable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: true,
        }
    }
}

pub(crate) const JOB_STATUS_QUEUED: &str = "queued";
pub(crate) const JOB_STATUS_RENDERING: &str = "rendering";
pub(crate) const JOB_STATUS_SUBMITTING: &str = "submitting";
pub(crate) const JOB_STATUS_PRINTING: &str = "printing";
pub(crate) const JOB_STATUS_NEEDS_ATTENTION: &str = "needs_attention";
pub(crate) const JOB_STATUS_SUCCEEDED: &str = "succeeded";
pub(crate) const JOB_STATUS_FAILED: &str = "failed";
pub(crate) const JOB_STATUS_CANCELED: &str = "canceled";
