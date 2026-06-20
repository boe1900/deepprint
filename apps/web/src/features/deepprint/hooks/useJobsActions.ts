import {
  useCallback,
  useEffect,
  useRef,
  type Dispatch,
  type FormEvent,
  type SetStateAction,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError, requestJson } from "../api";
import { createJobDetailQueryOptions } from "../queries";
import { useLatestAsyncTask } from "./useLatestAsyncTask";
import { appendTimelineEntry, clampPollInterval, isTerminalStatus } from "../utils";
import type {
  JobResponse,
  JobTimelineEntry,
  JobTimelineSource,
  NoticeState,
  RequestTimeoutSettings,
} from "../types";

interface UseJobsActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  jobIdInput: string;
  jobAutoPoll: boolean;
  jobPollIntervalSec: number;
  currentJobId: string | null | undefined;
  currentJobStatus: string | null | undefined;
  setJobLoading: (value: boolean) => void;
  setJobError: (value: string | null) => void;
  setJobPollError: (value: string | null) => void;
  setJobResult: (value: JobResponse | null) => void;
  setJobTimeline: Dispatch<SetStateAction<JobTimelineEntry[]>>;
  setNotice: (notice: NoticeState) => void;
}

interface UseJobsActionsResult {
  fetchJobById: (
    jobId: string,
    source: JobTimelineSource,
    silentOnError: boolean,
  ) => Promise<JobResponse | null>;
  onLookupJob: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  onRefreshCurrentJob: () => Promise<void>;
}

export function useJobsActions({
  baseUrl,
  requestTimeouts,
  jobIdInput,
  jobAutoPoll,
  jobPollIntervalSec,
  currentJobId,
  currentJobStatus,
  setJobLoading,
  setJobError,
  setJobPollError,
  setJobResult,
  setJobTimeline,
  setNotice,
}: UseJobsActionsArgs): UseJobsActionsResult {
  const queryClient = useQueryClient();
  const manualTask = useLatestAsyncTask();
  const pollTask = useLatestAsyncTask();
  const activeManualRef = useRef(false);
  const activePollRef = useRef(false);

  const fetchJobById = useCallback(
    async (
      jobId: string,
      source: JobTimelineSource,
      silentOnError: boolean,
    ): Promise<JobResponse | null> => {
      if (source === "poll") {
        if (activeManualRef.current || activePollRef.current) {
          return null;
        }
      } else {
        pollTask.cancel();
        activePollRef.current = false;
        setJobLoading(true);
      }

      const ticket = (source === "poll" ? pollTask : manualTask).start();
      if (source === "poll") {
        activePollRef.current = true;
      } else {
        activeManualRef.current = true;
      }

      try {
        const data = await queryClient.fetchQuery({
          ...createJobDetailQueryOptions({
            baseUrl,
            jobId,
            timeoutMs: requestTimeouts.jobStatus,
          }),
          queryFn: ({ signal }: { signal?: AbortSignal }) =>
            requestJson<JobResponse>(baseUrl, `/v1/jobs/${encodeURIComponent(jobId)}`, {
              signal: ticket.signal ?? signal,
              timeoutMs: requestTimeouts.jobStatus,
            }),
          staleTime: 0,
        });

        if (!ticket.isCurrent()) return null;

        setJobResult(data);
        setJobError(null);
        if (source === "poll") setJobPollError(null);
        setJobTimeline((previous) => appendTimelineEntry(previous, data, source));

        return data;
      } catch (error) {
        if (!ticket.isCurrent()) return null;
        if (isAbortError(error)) return null;

        const message = getRequestErrorMessage(error, tr("jobs.queryFailed"));
        if (source === "poll") {
          setJobPollError(message);
        } else {
          setJobError(message);
          if (!silentOnError) {
            setNotice({ kind: "error", message });
          }
        }

        return null;
      } finally {
        ticket.finish();

        if (source === "poll") {
          if (ticket.isCurrent()) {
            activePollRef.current = false;
          }
        } else {
          if (ticket.isCurrent()) {
            activeManualRef.current = false;
            setJobLoading(false);
          }
        }
      }
    },
    [
      baseUrl,
      manualTask,
      pollTask,
      queryClient,
      requestTimeouts.jobStatus,
      setJobError,
      setJobLoading,
      setJobPollError,
      setJobResult,
      setJobTimeline,
      setNotice,
    ],
  );

  const onLookupJob = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const jobId = jobIdInput.trim();
      if (!jobId) {
        setJobError(tr("writes.jobIdRequired"));
        setJobResult(null);
        return;
      }

      setJobError(null);
      setJobPollError(null);
      setJobTimeline([]);
      await fetchJobById(jobId, "manual", true);
    },
    [fetchJobById, jobIdInput, setJobError, setJobPollError, setJobResult, setJobTimeline],
  );

  const onRefreshCurrentJob = useCallback(async () => {
    if (!currentJobId) return;
    await fetchJobById(currentJobId, "manual", false);
  }, [currentJobId, fetchJobById]);

  useEffect(() => {
    if (!jobAutoPoll || !currentJobId || !currentJobStatus || isTerminalStatus(currentJobStatus)) {
      pollTask.cancel();
      activePollRef.current = false;
      return undefined;
    }

    const intervalSec = clampPollInterval(jobPollIntervalSec);
    const timer = window.setInterval(() => {
      void fetchJobById(currentJobId, "poll", true);
    }, intervalSec * 1000);

    return () => {
      window.clearInterval(timer);
      pollTask.cancel();
      activePollRef.current = false;
    };
  }, [currentJobId, currentJobStatus, fetchJobById, jobAutoPoll, jobPollIntervalSec, pollTask]);

  useEffect(
    () => () => {
      manualTask.cancel();
      pollTask.cancel();
      activeManualRef.current = false;
      activePollRef.current = false;
    },
    [manualTask, pollTask],
  );

  return {
    fetchJobById,
    onLookupJob,
    onRefreshCurrentJob,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
