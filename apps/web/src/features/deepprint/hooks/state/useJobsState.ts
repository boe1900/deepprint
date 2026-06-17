import { useState } from "react";
import { DEFAULT_JOB_POLL_INTERVAL_SEC } from "../../constants";
import type { JobResponse, JobTimelineEntry } from "../../types";

export function useJobsState() {
  const [jobIdInput, setJobIdInput] = useState("");
  const [jobResult, setJobResult] = useState<JobResponse | null>(null);
  const [jobLoading, setJobLoading] = useState(false);
  const [jobError, setJobError] = useState<string | null>(null);
  const [jobPollError, setJobPollError] = useState<string | null>(null);
  const [jobAutoPoll, setJobAutoPoll] = useState(true);
  const [jobPollIntervalSec, setJobPollIntervalSec] = useState(DEFAULT_JOB_POLL_INTERVAL_SEC);
  const [jobTimeline, setJobTimeline] = useState<JobTimelineEntry[]>([]);

  return {
    jobIdInput,
    setJobIdInput,
    jobResult,
    setJobResult,
    jobLoading,
    setJobLoading,
    jobError,
    setJobError,
    jobPollError,
    setJobPollError,
    jobAutoPoll,
    setJobAutoPoll,
    jobPollIntervalSec,
    setJobPollIntervalSec,
    jobTimeline,
    setJobTimeline,
  };
}
