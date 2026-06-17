import {
  DEFAULT_BASE_URL,
  DIAGNOSTICS_HISTORY_MAX_ITEMS,
  DIAGNOSTICS_HISTORY_STORAGE_KEY,
  JOB_STAGE_FLOW,
  MAX_JOB_POLL_INTERVAL_SEC,
  MIN_JOB_POLL_INTERVAL_SEC,
} from "./constants";
import type {
  DiagnosticHistoryItem,
  JobErrorCategory,
  JobResponse,
  JobTimelineEntry,
  JobTimelineSource,
} from "./types";

export function normalizeBaseUrl(raw: string): string {
  return raw.trim().replace(/\/+$/, "") || DEFAULT_BASE_URL;
}

export function formatPct(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function formatUnixSec(ts: number | null | undefined): string {
  if (!ts) return "-";
  return new Date(ts * 1000).toLocaleString();
}

export function formatUnixMs(ts: number | null | undefined): string {
  if (!ts) return "-";
  return new Date(ts).toLocaleString();
}

export function buildRequestId(): string {
  const randomPart = Math.random().toString(16).slice(2, 10);
  return `console-${Date.now()}-${randomPart}`;
}

export async function readFileAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error("读取文件失败：无法解析 DataURL"));
        return;
      }
      const marker = "base64,";
      const index = result.indexOf(marker);
      if (index < 0) {
        reject(new Error("读取文件失败：DataURL 缺少 base64 数据"));
        return;
      }
      resolve(result.slice(index + marker.length));
    };
    reader.onerror = () => reject(new Error("读取文件失败"));
    reader.readAsDataURL(file);
  });
}

export async function convertImageFileToPdf(file: File): Promise<File> {
  const dataUrl = await readFileAsDataUrl(file);
  const image = await loadImageFromUrl(dataUrl);
  const pageWidthPt = image.width * 0.75;
  const pageHeightPt = image.height * 0.75;
  const jpegBytes = await convertImageToJpegBytes(dataUrl, image.width, image.height);
  const contentStream = new TextEncoder().encode(
    `q\n${formatPdfNumber(pageWidthPt)} 0 0 ${formatPdfNumber(pageHeightPt)} 0 0 cm\n/Im0 Do\nQ\n`
  );

  const pdfBytes = buildPdfDocument([
    {
      dict: "<< /Type /Catalog /Pages 2 0 R >>",
    },
    {
      dict: "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
    },
    {
      dict: `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${formatPdfNumber(pageWidthPt)} ${formatPdfNumber(pageHeightPt)}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>`,
    },
    {
      dict: `<< /Type /XObject /Subtype /Image /Width ${image.width} /Height ${image.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpegBytes.length} >>`,
      stream: jpegBytes,
    },
    {
      dict: `<< /Length ${contentStream.length} >>`,
      stream: contentStream,
    },
  ]);
  const pdfName = file.name.replace(/\.[^.]+$/, "") || "image";
  const pdfBuffer = new ArrayBuffer(pdfBytes.byteLength);
  new Uint8Array(pdfBuffer).set(pdfBytes);
  return new File([pdfBuffer], `${pdfName}.pdf`, {
    type: "application/pdf",
    lastModified: file.lastModified,
  });
}

export function buildPdfObjectUrlFromBase64(payload: string): string {
  const binary = atob(payload);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return buildPdfObjectUrlFromBytes(bytes);
}

export function buildPdfObjectUrlFromBytes(bytes: Uint8Array): string {
  const normalized = new Uint8Array(bytes.byteLength);
  normalized.set(bytes);
  const blob = new Blob([normalized.buffer], { type: "application/pdf" });
  return URL.createObjectURL(blob);
}

export function buildObjectUrl(file: File): string {
  return URL.createObjectURL(file);
}

export function encodeBase64FromBytes(bytes: Uint8Array): string {
  let binary = "";
  const chunkSize = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += chunkSize) {
    const chunk = bytes.subarray(offset, offset + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result !== "string") {
        reject(new Error("读取图片失败"));
        return;
      }
      resolve(reader.result);
    };
    reader.onerror = () => reject(new Error("读取图片失败"));
    reader.readAsDataURL(file);
  });
}

function loadImageFromUrl(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error("图片解析失败"));
    image.src = url;
  });
}

async function convertImageToJpegBytes(
  url: string,
  width: number,
  height: number
): Promise<Uint8Array> {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("浏览器不支持 Canvas 绘制");
  }

  const image = await loadImageFromUrl(url);
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, width, height);
  context.drawImage(image, 0, 0, width, height);

  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob(
      (nextBlob) => {
        if (!nextBlob) {
          reject(new Error("图片转 PDF 失败：无法导出 JPEG"));
          return;
        }
        resolve(nextBlob);
      },
      "image/jpeg",
      0.92
    );
  });
  return new Uint8Array(await blob.arrayBuffer());
}

type PdfObject = {
  dict: string;
  stream?: Uint8Array;
};

function buildPdfDocument(objects: PdfObject[]): Uint8Array {
  const encoder = new TextEncoder();
  const chunks: Uint8Array[] = [];
  const offsets: number[] = [0];
  const header = encoder.encode("%PDF-1.4\n");
  chunks.push(header);
  let currentOffset = header.length;

  for (let index = 0; index < objects.length; index += 1) {
    offsets.push(currentOffset);
    const object = objects[index];
    const objectHeader = encoder.encode(`${index + 1} 0 obj\n${object.dict}\n`);
    chunks.push(objectHeader);
    currentOffset += objectHeader.length;

    if (object.stream) {
      const streamStart = encoder.encode("stream\n");
      const streamEnd = encoder.encode("\nendstream\n");
      chunks.push(streamStart);
      chunks.push(object.stream);
      chunks.push(streamEnd);
      currentOffset += streamStart.length + object.stream.length + streamEnd.length;
    }

    const objectEnd = encoder.encode("endobj\n");
    chunks.push(objectEnd);
    currentOffset += objectEnd.length;
  }

  const xrefOffset = currentOffset;
  const xrefLines = [`xref`, `0 ${objects.length + 1}`, `0000000000 65535 f `];
  for (let index = 1; index < offsets.length; index += 1) {
    xrefLines.push(`${String(offsets[index]).padStart(10, "0")} 00000 n `);
  }
  const trailer = [
    ...xrefLines,
    `trailer`,
    `<< /Size ${objects.length + 1} /Root 1 0 R >>`,
    `startxref`,
    `${xrefOffset}`,
    `%%EOF`,
  ].join("\n");
  chunks.push(encoder.encode(`${trailer}\n`));

  const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const output = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.length;
  }
  return output;
}

function formatPdfNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

export function loadDiagnosticsHistory(): DiagnosticHistoryItem[] {
  try {
    const raw = window.localStorage.getItem(DIAGNOSTICS_HISTORY_STORAGE_KEY);
    if (!raw) return [];

    const payload: unknown = JSON.parse(raw);
    if (!Array.isArray(payload)) return [];

    const output: DiagnosticHistoryItem[] = [];
    for (const entry of payload) {
      if (typeof entry !== "object" || entry === null) continue;
      const item = entry as Partial<DiagnosticHistoryItem>;
      if (
        typeof item.bundle_id !== "string" ||
        typeof item.bundle_path !== "string" ||
        typeof item.size_bytes !== "number" ||
        typeof item.created_at !== "number" ||
        typeof item.base_url !== "string" ||
        typeof item.exported_at_ms !== "number"
      ) {
        continue;
      }
      output.push(item as DiagnosticHistoryItem);
      if (output.length >= DIAGNOSTICS_HISTORY_MAX_ITEMS) break;
    }

    return output;
  } catch {
    return [];
  }
}

export function clampPollInterval(value: number): number {
  return Math.min(MAX_JOB_POLL_INTERVAL_SEC, Math.max(MIN_JOB_POLL_INTERVAL_SEC, value));
}

export function isTerminalStatus(status: string): boolean {
  return ["succeeded", "failed", "canceled", "dead_letter"].includes(status);
}

export function statusLabel(status: string): string {
  switch (status) {
    case "needs_attention":
      return "待确认";
    case "queued":
      return "排队";
    case "rendering":
      return "渲染中";
    case "submitting":
      return "提交中";
    case "printing":
      return "打印中";
    case "succeeded":
      return "成功";
    case "failed":
      return "失败";
    case "canceled":
      return "已取消";
    case "dead_letter":
      return "死信";
    default:
      return status;
  }
}

export function categorizeJobError(
  code: string | null,
  message: string | null,
): JobErrorCategory | null {
  if (!code && !message) return null;

  const normalizedCode = (code || "").toUpperCase();
  const normalizedMessage = (message || "").toUpperCase();

  if (normalizedCode.startsWith("RENDER_") || normalizedMessage.includes("TYPST")) {
    return {
      label: "渲染错误",
      level: "warn",
      hint: "检查模板语法、模板资源路径与数据字段映射。",
    };
  }

  if (
    normalizedCode.includes("TIMEOUT") ||
    normalizedMessage.includes("TIMEOUT") ||
    normalizedMessage.includes("TIMED OUT")
  ) {
    return {
      label: "超时错误",
      level: "warn",
      hint: "检查后端响应时延，必要时提升超时配置。",
    };
  }

  if (
    normalizedCode.startsWith("PRINT_") ||
    normalizedCode.startsWith("BACKEND_") ||
    normalizedMessage.includes("CUPS") ||
    normalizedMessage.includes("WINSPOOL")
  ) {
    return {
      label: "打印后端错误",
      level: "critical",
      hint: "检查打印机在线状态、驱动、队列权限与纸张配置。",
    };
  }

  if (normalizedCode.startsWith("AUTH_") || normalizedMessage.includes("UNAUTHORIZED")) {
    return {
      label: "鉴权错误",
      level: "warn",
      hint: "核对 token / secret / 时间戳窗口和签名算法。",
    };
  }

  if (normalizedCode.startsWith("DB_") || normalizedCode.startsWith("INTERNAL_")) {
    return {
      label: "系统错误",
      level: "critical",
      hint: "建议先导出诊断包，再排查数据库与日志。",
    };
  }

  if (normalizedCode.includes("CANCEL")) {
    return {
      label: "任务取消",
      level: "info",
      hint: "任务已被主动取消，如非预期请检查调用方行为。",
    };
  }

  return {
    label: "未知错误",
    level: "warn",
    hint: "建议查看错误码与诊断包详情定位根因。",
  };
}

export function appendTimelineEntry(
  previous: JobTimelineEntry[],
  job: JobResponse,
  source: JobTimelineSource,
): JobTimelineEntry[] {
  const entry: JobTimelineEntry = {
    status: job.status,
    attempt_count: job.attempt_count,
    updated_at: job.updated_at,
    last_error_code: job.last_error_code,
    last_error_message: job.last_error_message,
    source,
  };

  const last = previous[previous.length - 1];
  if (
    last &&
    last.status === entry.status &&
    last.attempt_count === entry.attempt_count &&
    last.updated_at === entry.updated_at &&
    last.last_error_code === entry.last_error_code &&
    last.last_error_message === entry.last_error_message
  ) {
    return previous;
  }

  return [...previous, entry];
}

export function statusStageIndex(status: string): number {
  return JOB_STAGE_FLOW.indexOf(status as (typeof JOB_STAGE_FLOW)[number]);
}
