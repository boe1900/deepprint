import { useCallback, type Dispatch, type FormEvent, type SetStateAction } from "react";
import {
  getRequestErrorMessage,
  isAbortError,
  requestJson,
  saveDiagnosticBundle,
  signClientWriteHeaders,
} from "../api";
import {
  DEFAULT_CREATE_DATA_JSON,
  DEFAULT_CREATE_TEMPLATE,
  DIAGNOSTICS_HISTORY_MAX_ITEMS,
} from "../constants";
import { useLatestAsyncTask } from "./useLatestAsyncTask";
import { buildRequestId, normalizeBaseUrl } from "../utils";
import {
  buildTemplatePrintOptions as buildTemplatePrintOptionsFromInput,
  parseTemplateDataJson,
  resolveTemplateContent as resolveTemplateContentFromInput,
} from "./writesHelpers";
import {
  type CreateJobInput,
  type PreviewTypstInput,
  useWritesCreatePreviewActions,
} from "./useWritesCreatePreviewActions";
import {
  type CreateDirectJobInput,
  useWritesDirectCancelActions,
} from "./useWritesDirectCancelActions";
import type {
  CancelJobResponse,
  CreateJobResponse,
  DiagnosticExportResponse,
  DiagnosticHistoryItem,
  JobResponse,
  JobTimelineEntry,
  JobTimelineSource,
  NoticeState,
  PreviewTypstResponse,
  RequestTimeoutSettings,
} from "../types";

interface UseWritesActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  authRequiredForWrites: boolean;
  writeAuthToken: string;
  writeAuthSecret: string;

  createRequestId: string;
  createTemplateContent: string;
  createDataJson: string;
  createPrinterId: string;
  createCopies: string;
  createPaperSize: string;
  createDuplex: string;

  directPrinterId: string;
  directSelectedFile: File | null;
  directJobMaxBytes: number | null | undefined;

  cancelTargetJobId: string;
  currentJobId: string | null | undefined;
  latestCreatedJobId: string | null | undefined;
  jobIdInput: string;

  fetchJobById: (
    jobId: string,
    source: JobTimelineSource,
    silentOnError: boolean,
  ) => Promise<JobResponse | null>;

  setNotice: (notice: NoticeState) => void;
  setJobIdInput: (value: string) => void;
  setJobTimeline: Dispatch<SetStateAction<JobTimelineEntry[]>>;
  setCancelTargetJobId: (value: string) => void;

  setCreateRequestId: (value: string) => void;
  setCreateTemplateContent: (value: string) => void;
  setCreateDataJson: (value: string) => void;
  setCreatePrinterId: (value: string) => void;
  setCreateCopies: (value: string) => void;
  setCreatePaperSize: (value: string) => void;
  setCreateDuplex: (value: string) => void;
  setCreateLoading: (value: boolean) => void;
  setCreateError: (value: string | null) => void;
  setCreateResult: (value: CreateJobResponse | null) => void;

  setPreviewLoading: (value: boolean) => void;
  setPreviewError: (value: string | null) => void;
  setPreviewResult: (value: PreviewTypstResponse | null) => void;
  setPreviewPdfUrl: Dispatch<SetStateAction<string | null>>;
  setPreviewModalOpen: (value: boolean) => void;

  setDirectPrinterId: (value: string) => void;
  setDirectSelectedFile: (value: File | null) => void;
  setDirectFileInputKey: Dispatch<SetStateAction<number>>;
  setDirectLoading: (value: boolean) => void;
  setDirectError: (value: string | null) => void;
  setDirectResult: (value: CreateJobResponse | null) => void;

  setCancelLoading: (value: boolean) => void;
  setCancelError: (value: string | null) => void;
  setCancelResult: (value: CancelJobResponse | null) => void;

  setDiagLoading: (value: boolean) => void;
  setDiagResult: (value: DiagnosticExportResponse | null) => void;
  setDiagHistory: Dispatch<SetStateAction<DiagnosticHistoryItem[]>>;
}

interface UseWritesActionsResult {
  onCreateJob: (
    event: FormEvent<HTMLFormElement>,
    input?: CreateJobInput,
  ) => Promise<void>;
  onPreviewTypst: (input?: PreviewTypstInput) => Promise<void>;
  onDismissPreview: () => void;
  onCreateDirectJob: (
    event: FormEvent<HTMLFormElement>,
    input?: CreateDirectJobInput,
  ) => Promise<void>;
  onCancelJob: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  onExportDiagnostics: () => Promise<void>;
  onResetWriteForms: () => void;
}

export function useWritesActions({
  baseUrl,
  requestTimeouts,
  authRequiredForWrites,
  writeAuthToken,
  writeAuthSecret,

  createRequestId,
  createTemplateContent,
  createDataJson,
  createPrinterId,
  createCopies,
  createPaperSize,
  createDuplex,

  directPrinterId,
  directSelectedFile,
  directJobMaxBytes,

  cancelTargetJobId,
  currentJobId,
  latestCreatedJobId,
  jobIdInput,

  fetchJobById,

  setNotice,
  setJobIdInput,
  setJobTimeline,
  setCancelTargetJobId,

  setCreateRequestId,
  setCreateTemplateContent,
  setCreateDataJson,
  setCreatePrinterId,
  setCreateCopies,
  setCreatePaperSize,
  setCreateDuplex,
  setCreateLoading,
  setCreateError,
  setCreateResult,

  setPreviewLoading,
  setPreviewError,
  setPreviewResult,
  setPreviewPdfUrl,
  setPreviewModalOpen,

  setDirectPrinterId,
  setDirectSelectedFile,
  setDirectFileInputKey,
  setDirectLoading,
  setDirectError,
  setDirectResult,

  setCancelLoading,
  setCancelError,
  setCancelResult,

  setDiagLoading,
  setDiagResult,
  setDiagHistory,
}: UseWritesActionsArgs): UseWritesActionsResult {
  const createTask = useLatestAsyncTask();
  const previewTask = useLatestAsyncTask();
  const directTask = useLatestAsyncTask();
  const cancelTask = useLatestAsyncTask();
  const exportTask = useLatestAsyncTask();

  const prepareWriteHeaders = useCallback(
    async (method: string, path: string, bodyText: string): Promise<Record<string, string>> => {
      const token = writeAuthToken.trim();
      const secret = writeAuthSecret.trim();
      const hasAnyAuthInput = token.length > 0 || secret.length > 0;

      if (!authRequiredForWrites && !hasAnyAuthInput) {
        return {};
      }

      if (hasAnyAuthInput && (!token || !secret)) {
        throw new Error("若要发送签名请求，请同时填写 token 与 secret");
      }

      try {
        return await signClientWriteHeaders(method, path, bodyText, token || null, secret || null);
      } catch (error) {
        const message = error instanceof Error ? error.message : "签名失败";
        if (authRequiredForWrites) {
          throw new Error(
            `当前 Agent 已启用操作鉴权，签名失败：${message}。请确认初始化向导已保存凭据，或在当前页面输入 token 与 secret。`,
          );
        }
        throw new Error(`签名失败：${message}`);
      }
    },
    [authRequiredForWrites, writeAuthSecret, writeAuthToken],
  );

  const buildTemplatePrintOptions = useCallback((): Record<string, unknown> => {
    return buildTemplatePrintOptionsFromInput({
      copies: createCopies,
      paperSize: createPaperSize,
      duplex: createDuplex,
    });
  }, [createCopies, createDuplex, createPaperSize]);

  const buildDirectPrintOptions = useCallback((): Record<string, unknown> => {
    return { copies: 1 };
  }, []);

  const updatePreviewPdfUrl = useCallback(
    (nextUrl: string | null) => {
      setPreviewPdfUrl((previous) => {
        if (previous) {
          URL.revokeObjectURL(previous);
        }
        return nextUrl;
      });
    },
    [setPreviewPdfUrl],
  );

  const parseCreateData = useCallback((): unknown => {
    return parseTemplateDataJson(createDataJson);
  }, [createDataJson]);

  const resolveTemplateContent = useCallback((): string => {
    return resolveTemplateContentFromInput(createTemplateContent);
  }, [createTemplateContent]);

  const syncCreatedJobToQuery = useCallback(
    async (jobId: string) => {
      setCancelTargetJobId(jobId);
      setJobIdInput(jobId);
      setJobTimeline([]);
      await fetchJobById(jobId, "manual", true);
    },
    [fetchJobById, setCancelTargetJobId, setJobIdInput, setJobTimeline],
  );

  const { onCreateJob, onPreviewTypst, onDismissPreview } = useWritesCreatePreviewActions({
    baseUrl,
    writesTimeoutMs: requestTimeouts.writes,
    createRequestId,
    createPrinterId,
    createTemplateContent,
    createTask,
    previewTask,
    parseCreateData,
    resolveTemplateContent,
    buildTemplatePrintOptions,
    prepareWriteHeaders,
    syncCreatedJobToQuery,
    updatePreviewPdfUrl,
    setNotice,
    setCreateRequestId,
    setCreateLoading,
    setCreateError,
    setCreateResult,
    setPreviewModalOpen,
    setPreviewLoading,
    setPreviewError,
    setPreviewResult,
  });

  const { onCreateDirectJob, onCancelJob } = useWritesDirectCancelActions({
    baseUrl,
    writesTimeoutMs: requestTimeouts.writes,
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
  });

  const onExportDiagnostics = useCallback(async () => {
    const ticket = exportTask.start();
    setDiagLoading(true);
    try {
      const path = "/v1/diagnostics/export";
      const bodyText = JSON.stringify({});
      const headers = await prepareWriteHeaders("POST", path, bodyText);
      const data = await requestJson<DiagnosticExportResponse>(baseUrl, path, {
        method: "POST",
        body: bodyText,
        headers,
        signal: ticket.signal,
        timeoutMs: requestTimeouts.diagnosticsExport,
      });

      if (!ticket.isCurrent()) return;

      let exportResult: DiagnosticExportResponse | null = data;
      let appendHistory = true;
      let exportNotice = "诊断包导出成功";
      try {
        const saved = await saveDiagnosticBundle(
          data.bundle_path,
          `${data.bundle_id}.zip`,
          true,
          true,
        );
        if (!ticket.isCurrent()) return;

        if (saved.saved) {
          if (saved.destination_path) {
            exportResult = { ...data, bundle_path: saved.destination_path };
          }
          exportNotice = saved.source_deleted
            ? "诊断包已保存到本地并清理临时文件"
            : "诊断包已保存到本地";
        } else {
          appendHistory = false;
          exportResult = null;
          exportNotice = saved.source_deleted
            ? "已取消保存，临时诊断包已清理"
            : "已取消保存诊断包保存";
        }
      } catch (error) {
        if (!ticket.isCurrent()) return;

        const message = error instanceof Error ? error.message : "另存为失败";
        exportNotice = `诊断包已导出（另存为失败: ${message}）`;
      }

      if (!ticket.isCurrent()) return;

      setDiagResult(exportResult);
      if (appendHistory && exportResult) {
        const historyItem: DiagnosticHistoryItem = {
          ...exportResult,
          base_url: normalizeBaseUrl(baseUrl),
          exported_at_ms: Date.now(),
        };
        setDiagHistory((previous) =>
          [
            historyItem,
            ...previous.filter(
              (item) =>
                !(
                  item.bundle_id === historyItem.bundle_id && item.base_url === historyItem.base_url
                ),
            ),
          ].slice(0, DIAGNOSTICS_HISTORY_MAX_ITEMS),
        );
      }
      setNotice({ kind: "ok", message: exportNotice });
    } catch (error) {
      if (!ticket.isCurrent()) return;
      if (isAbortError(error)) return;

      const message = getRequestErrorMessage(error, "诊断导出失败");
      setNotice({ kind: "error", message });
    } finally {
      ticket.finish();
      if (ticket.isCurrent()) {
        setDiagLoading(false);
      }
    }
  }, [
    baseUrl,
    exportTask,
    prepareWriteHeaders,
    requestTimeouts.diagnosticsExport,
    setDiagHistory,
    setDiagLoading,
    setDiagResult,
    setNotice,
  ]);

  const onResetWriteForms = useCallback(() => {
    createTask.cancel();
    previewTask.cancel();
    directTask.cancel();
    cancelTask.cancel();
    exportTask.cancel();

    setCreateLoading(false);
    setPreviewLoading(false);
    setDirectLoading(false);
    setCancelLoading(false);

    setCreateRequestId(buildRequestId());
    setCreateTemplateContent(DEFAULT_CREATE_TEMPLATE);
    setCreateDataJson(DEFAULT_CREATE_DATA_JSON);
    setCreatePrinterId("");
    setCreateCopies("1");
    setCreatePaperSize("");
    setCreateDuplex("");
    setDirectPrinterId("");
    setDirectSelectedFile(null);
    setDirectFileInputKey((value) => value + 1);
    setDirectResult(null);
    setDirectError(null);
    setCancelTargetJobId("");
    setCreateResult(null);
    setPreviewModalOpen(false);
    setPreviewResult(null);
    setPreviewError(null);
    updatePreviewPdfUrl(null);
    setCancelResult(null);
    setCreateError(null);
    setCancelError(null);
    setNotice({ kind: "ok", message: "已重置打印操作表单" });
  }, [
    cancelTask,
    createTask,
    directTask,
    exportTask,
    previewTask,
    setCancelLoading,
    setCancelError,
    setCancelResult,
    setCancelTargetJobId,
    setCreateLoading,
    setCreateCopies,
    setCreateDataJson,
    setCreateDuplex,
    setCreateError,
    setCreatePaperSize,
    setCreatePrinterId,
    setCreateRequestId,
    setCreateResult,
    setCreateTemplateContent,
    setDirectError,
    setDirectFileInputKey,
    setDirectLoading,
    setDirectPrinterId,
    setDirectResult,
    setDirectSelectedFile,
    setNotice,
    setPreviewError,
    setPreviewLoading,
    setPreviewModalOpen,
    setPreviewResult,
    updatePreviewPdfUrl,
  ]);

  return {
    onCreateJob,
    onPreviewTypst,
    onDismissPreview,
    onCreateDirectJob,
    onCancelJob,
    onExportDiagnostics,
    onResetWriteForms,
  };
}
