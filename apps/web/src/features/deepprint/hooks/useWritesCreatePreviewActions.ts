import { useCallback, type FormEvent } from "react";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError, requestBinary, requestJson } from "../api";
import { formatTypstPreviewError } from "../typstPreviewError";
import { buildPdfObjectUrlFromBytes, buildRequestId } from "../utils";
import type { CreateJobResponse, NoticeState, PreviewTypstResponse } from "../types";
import type { useLatestAsyncTask } from "./useLatestAsyncTask";
import {
  parsePreviewOptionalNumber,
  parsePreviewRequiredNumber,
  parseTemplateDataJson,
  readPreviewRequiredHeader,
} from "./writesHelpers";

type LatestAsyncTask = ReturnType<typeof useLatestAsyncTask>;

interface UseWritesCreatePreviewActionsArgs {
  baseUrl: string;
  writesTimeoutMs: number;
  createRequestId: string;
  createPrinterId: string;
  createTemplateContent: string;
  createTask: LatestAsyncTask;
  previewTask: LatestAsyncTask;
  parseCreateData: () => unknown;
  resolveTemplateContent: () => string;
  buildTemplatePrintOptions: () => Record<string, unknown>;
  prepareWriteHeaders: (
    method: string,
    path: string,
    bodyText: string,
  ) => Promise<Record<string, string>>;
  syncCreatedJobToQuery: (jobId: string) => Promise<void>;
  updatePreviewPdfUrl: (nextUrl: string | null) => void;
  setNotice: (notice: NoticeState) => void;
  setCreateRequestId: (value: string) => void;
  setCreateLoading: (value: boolean) => void;
  setCreateError: (value: string | null) => void;
  setCreateResult: (value: CreateJobResponse | null) => void;
  setPreviewModalOpen: (value: boolean) => void;
  setPreviewLoading: (value: boolean) => void;
  setPreviewError: (value: string | null) => void;
  setPreviewResult: (value: PreviewTypstResponse | null) => void;
}

interface UseWritesCreatePreviewActionsResult {
  onCreateJob: (
    event: FormEvent<HTMLFormElement>,
    input?: CreateJobInput,
  ) => Promise<void>;
  onPreviewTypst: (input?: PreviewTypstInput) => Promise<void>;
  onDismissPreview: () => void;
}

export type CreateJobInput = {
  printOptions?: Record<string, unknown>;
};

export type PreviewTypstInput = {
  templateContent?: string;
  dataJson?: string;
  printOptions?: Record<string, unknown>;
};

const PREVIEW_HEADER_OUTPUT_KIND = "x-deepprint-preview-output-kind";
const PREVIEW_HEADER_PAGE_COUNT = "x-deepprint-preview-page-count";
const PREVIEW_HEADER_PAGE_WIDTH_PT = "x-deepprint-preview-page-width-pt";
const PREVIEW_HEADER_PAGE_HEIGHT_PT = "x-deepprint-preview-page-height-pt";

export function useWritesCreatePreviewActions({
  baseUrl,
  writesTimeoutMs,
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
}: UseWritesCreatePreviewActionsArgs): UseWritesCreatePreviewActionsResult {
  const onCreateJob = useCallback(
    async (event: FormEvent<HTMLFormElement>, input?: CreateJobInput) => {
      event.preventDefault();
      const ticket = createTask.start();
      setCreateLoading(true);
      setCreateError(null);
      try {
        const nextRequestIdValue = createRequestId.trim() || buildRequestId();
        if (!createRequestId.trim()) {
          setCreateRequestId(nextRequestIdValue);
        }

        const templateContent = resolveTemplateContent();
        const data = parseCreateData();
        const printerId = createPrinterId.trim();
        if (!printerId) {
          throw new Error(tr("writes.selectManagedPrinter"));
        }

        const payload = {
          request_id: nextRequestIdValue,
          printer_id: printerId,
          template_content: templateContent,
          data,
          print_options: input?.printOptions ?? buildTemplatePrintOptions(),
        };
        const bodyText = JSON.stringify(payload);
        const path = "/v1/jobs";
        const headers = await prepareWriteHeaders("POST", path, bodyText);

        const result = await requestJson<CreateJobResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: writesTimeoutMs,
        });

        if (!ticket.isCurrent()) return;

        setCreateResult(result);
        await syncCreatedJobToQuery(result.job_id);

        if (!ticket.isCurrent()) return;

        const message = result.idempotent
          ? tr("writes.createIdempotent", { jobId: result.job_id })
          : tr("writes.createSucceeded", { jobId: result.job_id });
        setNotice({ kind: "ok", message });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;
        const message = getRequestErrorMessage(error, tr("writes.createFailed"));
        setCreateError(message);
        setNotice({ kind: "error", message: tr("writes.createFailedWithMessage", { message }) });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setCreateLoading(false);
        }
      }
    },
    [
      baseUrl,
      createRequestId,
      createPrinterId,
      createTask,
      buildTemplatePrintOptions,
      parseCreateData,
      prepareWriteHeaders,
      resolveTemplateContent,
      setCreateError,
      setCreateLoading,
      setCreateRequestId,
      setCreateResult,
      setNotice,
      syncCreatedJobToQuery,
      writesTimeoutMs,
    ],
  );

  const onPreviewTypst = useCallback(async (input?: PreviewTypstInput) => {
    const ticket = previewTask.start();
    setPreviewModalOpen(true);
    setPreviewLoading(true);
    setPreviewError(null);
    setPreviewResult(null);
    updatePreviewPdfUrl(null);
    try {
      const templateContent =
        input?.templateContent != null
          ? input.templateContent.trim()
          : resolveTemplateContent();
      if (!templateContent) {
        throw new Error(tr("writes.templateContentRequired"));
      }
      const data =
        input?.dataJson != null
          ? parseTemplateDataJson(input.dataJson)
          : parseCreateData();

      const payload = {
        template_content: templateContent,
        data,
        print_options: input?.printOptions ?? buildTemplatePrintOptions(),
      };
      const path = "/v1/preview/typst";
      const bodyText = JSON.stringify(payload);
      const headers = await prepareWriteHeaders("POST", path, bodyText);
      const response = await requestBinary(baseUrl, path, {
        method: "POST",
        body: bodyText,
        headers,
        signal: ticket.signal,
        timeoutMs: writesTimeoutMs,
      });

      if (!ticket.isCurrent()) return;

      const contentType = response.contentType.toLowerCase();
      if (!contentType.includes("application/pdf")) {
        throw new Error(tr("writes.previewNotPdf", { contentType: response.contentType || "unknown" }));
      }

      const result: PreviewTypstResponse = {
        output_kind: readPreviewRequiredHeader(response.headers, PREVIEW_HEADER_OUTPUT_KIND),
        page_count: Math.max(
          1,
          Math.round(parsePreviewRequiredNumber(response.headers, PREVIEW_HEADER_PAGE_COUNT)),
        ),
        page_width_pt: parsePreviewOptionalNumber(response.headers, PREVIEW_HEADER_PAGE_WIDTH_PT),
        page_height_pt: parsePreviewOptionalNumber(response.headers, PREVIEW_HEADER_PAGE_HEIGHT_PT),
      };

      const url = buildPdfObjectUrlFromBytes(response.payload);
      updatePreviewPdfUrl(url);
      setPreviewResult(result);
    } catch (error) {
      if (!ticket.isCurrent()) return;
      if (isAbortError(error)) return;

      const rawMessage = getRequestErrorMessage(error, tr("writes.previewFailed"));
      const message = formatTypstPreviewError(
        rawMessage,
        input?.templateContent ?? createTemplateContent,
      );
      setPreviewError(message);
    } finally {
      ticket.finish();
      if (ticket.isCurrent()) {
        setPreviewLoading(false);
      }
    }
  }, [
    baseUrl,
    buildTemplatePrintOptions,
    createTemplateContent,
    parseCreateData,
    prepareWriteHeaders,
    previewTask,
    resolveTemplateContent,
    setPreviewError,
    setPreviewLoading,
    setPreviewModalOpen,
    setPreviewResult,
    updatePreviewPdfUrl,
    writesTimeoutMs,
  ]);

  const onDismissPreview = useCallback(() => {
    previewTask.cancel();
    setPreviewModalOpen(false);
    setPreviewLoading(false);
    setPreviewError(null);
    setPreviewResult(null);
    updatePreviewPdfUrl(null);
  }, [
    previewTask,
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
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
