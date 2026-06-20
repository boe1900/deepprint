import { useCallback, type FormEvent } from "react";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError, requestJson } from "../api";
import {
  buildRequestId,
  formatBytes,
  readFileAsBase64,
} from "../utils";
import type { CancelJobResponse, CreateJobResponse, JobResponse, NoticeState } from "../types";
import type { useLatestAsyncTask } from "./useLatestAsyncTask";

type LatestAsyncTask = ReturnType<typeof useLatestAsyncTask>;

interface UseWritesDirectCancelActionsArgs {
  baseUrl: string;
  writesTimeoutMs: number;
  directTask: LatestAsyncTask;
  cancelTask: LatestAsyncTask;
  directPrinterId: string;
  directSelectedFile: File | null;
  directJobMaxBytes: number | null | undefined;
  buildDirectPrintOptions: () => Record<string, unknown>;
  cancelTargetJobId: string;
  currentJobId: string | null | undefined;
  latestCreatedJobId: string | null | undefined;
  jobIdInput: string;
  prepareWriteHeaders: (
    method: string,
    path: string,
    bodyText: string,
  ) => Promise<Record<string, string>>;
  syncCreatedJobToQuery: (jobId: string) => Promise<void>;
  fetchJobById: (
    jobId: string,
    source: "manual" | "poll",
    silentOnError: boolean,
  ) => Promise<JobResponse | null>;
  setNotice: (notice: NoticeState) => void;
  setJobIdInput: (value: string) => void;
  setCancelTargetJobId: (value: string) => void;
  setDirectLoading: (value: boolean) => void;
  setDirectError: (value: string | null) => void;
  setDirectResult: (value: CreateJobResponse | null) => void;
  setCancelLoading: (value: boolean) => void;
  setCancelError: (value: string | null) => void;
  setCancelResult: (value: CancelJobResponse | null) => void;
}

interface UseWritesDirectCancelActionsResult {
  onCreateDirectJob: (
    event: FormEvent<HTMLFormElement>,
    input?: CreateDirectJobInput,
  ) => Promise<void>;
  onCancelJob: (event: FormEvent<HTMLFormElement>) => Promise<void>;
}

export type CreateDirectJobInput = {
  file?: File;
  printOptions?: Record<string, unknown>;
};

export function useWritesDirectCancelActions({
  baseUrl,
  writesTimeoutMs,
  directTask,
  cancelTask,
  directPrinterId,
  directSelectedFile,
  directJobMaxBytes,
  buildDirectPrintOptions,
  cancelTargetJobId,
  currentJobId,
  latestCreatedJobId,
  jobIdInput,
  prepareWriteHeaders,
  syncCreatedJobToQuery,
  fetchJobById,
  setNotice,
  setJobIdInput,
  setCancelTargetJobId,
  setDirectLoading,
  setDirectError,
  setDirectResult,
  setCancelLoading,
  setCancelError,
  setCancelResult,
}: UseWritesDirectCancelActionsArgs): UseWritesDirectCancelActionsResult {
  const onCreateDirectJob = useCallback(
    async (event: FormEvent<HTMLFormElement>, input?: CreateDirectJobInput) => {
      event.preventDefault();
      const ticket = directTask.start();
      setDirectLoading(true);
      setDirectError(null);
      try {
        const nextRequestIdValue = buildRequestId();

        const file = input?.file ?? directSelectedFile;
        if (!file) {
          throw new Error(tr("writes.selectFile"));
        }

        if (
          typeof directJobMaxBytes === "number" &&
          directJobMaxBytes > 0 &&
          file.size > directJobMaxBytes
        ) {
          throw new Error(
            tr("writes.fileTooLarge", {
              size: formatBytes(file.size),
              limit: formatBytes(directJobMaxBytes),
            }),
          );
        }

        const printerId = directPrinterId.trim();
        if (!printerId) {
          throw new Error(tr("writes.selectManagedPrinter"));
        }

        const printOptions = input?.printOptions ?? buildDirectPrintOptions();

        const fileContentBase64 = await readFileAsBase64(file);
        const path = "/v1/jobs/direct";
        const payload = {
          request_id: nextRequestIdValue,
          printer_id: printerId,
          file_name: file.name,
          file_content_base64: fileContentBase64,
          content_type: file.type || undefined,
          print_options: printOptions,
        };
        const bodyText = JSON.stringify(payload);
        const headers = await prepareWriteHeaders("POST", path, bodyText);
        const result = await requestJson<CreateJobResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: writesTimeoutMs,
        });

        if (!ticket.isCurrent()) return;

        setDirectResult(result);
        await syncCreatedJobToQuery(result.job_id);

        if (!ticket.isCurrent()) return;

        const message = result.idempotent
          ? tr("writes.createDirectIdempotent", { jobId: result.job_id })
          : tr("writes.createDirectSucceeded", { jobId: result.job_id });
        setNotice({ kind: "ok", message });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("writes.createDirectFailed"));
        setDirectError(message);
        setNotice({ kind: "error", message: tr("writes.createDirectFailedWithMessage", { message }) });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setDirectLoading(false);
        }
      }
    },
    [
      baseUrl,
      buildDirectPrintOptions,
      directJobMaxBytes,
      directPrinterId,
      directSelectedFile,
      directTask,
      prepareWriteHeaders,
      setDirectError,
      setDirectLoading,
      setDirectResult,
      setNotice,
      syncCreatedJobToQuery,
      writesTimeoutMs,
    ],
  );

  const onCancelJob = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const ticket = cancelTask.start();
      setCancelLoading(true);
      setCancelError(null);
      try {
        const targetJobId =
          cancelTargetJobId.trim() || currentJobId || latestCreatedJobId || jobIdInput.trim();
        if (!targetJobId) {
          throw new Error(tr("writes.jobIdRequired"));
        }

        const encodedJobId = encodeURIComponent(targetJobId);
        const path = `/v1/jobs/${encodedJobId}/cancel`;
        const bodyText = JSON.stringify({});
        const headers = await prepareWriteHeaders("POST", path, bodyText);

        const result = await requestJson<CancelJobResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: writesTimeoutMs,
        });

        if (!ticket.isCurrent()) return;

        setCancelResult(result);
        setCancelTargetJobId(result.job_id);
        setJobIdInput(result.job_id);
        await fetchJobById(result.job_id, "manual", true);

        if (!ticket.isCurrent()) return;

        setNotice({ kind: "ok", message: tr("writes.cancelSubmitted", { jobId: result.job_id }) });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("writes.cancelFailed"));
        setCancelError(message);
        setNotice({ kind: "error", message: tr("writes.cancelFailedWithMessage", { message }) });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setCancelLoading(false);
        }
      }
    },
    [
      baseUrl,
      cancelTargetJobId,
      cancelTask,
      currentJobId,
      fetchJobById,
      jobIdInput,
      latestCreatedJobId,
      prepareWriteHeaders,
      setCancelError,
      setCancelLoading,
      setCancelResult,
      setCancelTargetJobId,
      setJobIdInput,
      setNotice,
      writesTimeoutMs,
    ],
  );

  return {
    onCreateDirectJob,
    onCancelJob,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
