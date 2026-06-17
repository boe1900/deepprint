import { useState } from "react";
import { DEFAULT_CREATE_DATA_JSON, DEFAULT_CREATE_TEMPLATE } from "../../constants";
import { buildRequestId } from "../../utils";
import type { CancelJobResponse, CreateJobResponse, PreviewTypstResponse } from "../../types";

export function useWritesState() {
  const [writeAuthToken, setWriteAuthToken] = useState("");
  const [writeAuthSecret, setWriteAuthSecret] = useState("");

  const [createRequestId, setCreateRequestId] = useState(() => buildRequestId());
  const [createTemplateContent, setCreateTemplateContent] = useState(DEFAULT_CREATE_TEMPLATE);
  const [createDataJson, setCreateDataJson] = useState(DEFAULT_CREATE_DATA_JSON);
  const [createPrinterId, setCreatePrinterId] = useState("");
  const [createCopies, setCreateCopies] = useState("1");
  const [createPaperSize, setCreatePaperSize] = useState("");
  const [createDuplex, setCreateDuplex] = useState("");
  const [createLoading, setCreateLoading] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [createResult, setCreateResult] = useState<CreateJobResponse | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewResult, setPreviewResult] = useState<PreviewTypstResponse | null>(null);
  const [previewPdfUrl, setPreviewPdfUrl] = useState<string | null>(null);
  const [previewModalOpen, setPreviewModalOpen] = useState(false);

  const [directPrinterId, setDirectPrinterId] = useState("");
  const [directSelectedFile, setDirectSelectedFile] = useState<File | null>(null);
  const [directFileInputKey, setDirectFileInputKey] = useState(0);
  const [directLoading, setDirectLoading] = useState(false);
  const [directError, setDirectError] = useState<string | null>(null);
  const [directResult, setDirectResult] = useState<CreateJobResponse | null>(null);

  const [cancelTargetJobId, setCancelTargetJobId] = useState("");
  const [cancelLoading, setCancelLoading] = useState(false);
  const [cancelError, setCancelError] = useState<string | null>(null);
  const [cancelResult, setCancelResult] = useState<CancelJobResponse | null>(null);

  return {
    writeAuthToken,
    setWriteAuthToken,
    writeAuthSecret,
    setWriteAuthSecret,

    createRequestId,
    setCreateRequestId,
    createTemplateContent,
    setCreateTemplateContent,
    createDataJson,
    setCreateDataJson,
    createPrinterId,
    setCreatePrinterId,
    createCopies,
    setCreateCopies,
    createPaperSize,
    setCreatePaperSize,
    createDuplex,
    setCreateDuplex,
    createLoading,
    setCreateLoading,
    createError,
    setCreateError,
    createResult,
    setCreateResult,
    previewLoading,
    setPreviewLoading,
    previewError,
    setPreviewError,
    previewResult,
    setPreviewResult,
    previewPdfUrl,
    setPreviewPdfUrl,
    previewModalOpen,
    setPreviewModalOpen,

    directPrinterId,
    setDirectPrinterId,
    directSelectedFile,
    setDirectSelectedFile,
    directFileInputKey,
    setDirectFileInputKey,
    directLoading,
    setDirectLoading,
    directError,
    setDirectError,
    directResult,
    setDirectResult,

    cancelTargetJobId,
    setCancelTargetJobId,
    cancelLoading,
    setCancelLoading,
    cancelError,
    setCancelError,
    cancelResult,
    setCancelResult,
  };
}
