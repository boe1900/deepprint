import { getCurrentLocale, translate } from "@/i18n";

const RENDER_CODE_REGEX = /\[(RENDER_[A-Z_]+)\]/;
const SOURCE_DIAGNOSTIC_MESSAGE_REGEX = /message:\s*"([^"]+)"/g;

type TypstRenderCode =
  | "RENDER_TEMPLATE_SYNTAX"
  | "RENDER_TEMPLATE_SEMANTIC"
  | "RENDER_RESOURCE_MISSING"
  | "RENDER_COMPILE_FAILED";

function extractRenderCode(rawMessage: string): TypstRenderCode | null {
  const matched = rawMessage.match(RENDER_CODE_REGEX)?.[1];
  if (
    matched === "RENDER_TEMPLATE_SYNTAX" ||
    matched === "RENDER_TEMPLATE_SEMANTIC" ||
    matched === "RENDER_RESOURCE_MISSING" ||
    matched === "RENDER_COMPILE_FAILED"
  ) {
    return matched;
  }
  return null;
}

function extractUniqueSourceDiagnostics(rawMessage: string): string[] {
  const unique = new Set<string>();
  let matched: RegExpExecArray | null = null;
  while ((matched = SOURCE_DIAGNOSTIC_MESSAGE_REGEX.exec(rawMessage)) !== null) {
    const message = matched[1]?.trim();
    if (message) {
      unique.add(message);
    }
  }
  return Array.from(unique);
}

function formatDiagnosticsLines(rawMessage: string): string[] {
  const diagnostics = extractUniqueSourceDiagnostics(rawMessage).slice(0, 3);
  return diagnostics.map((message, index) => `${index + 1}. ${message}`);
}

function buildTemplateErrorMessage(rawMessage: string, templateContent: string): string {
  const lines = [tr("typstPreview.templateInvalid")];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push(tr("typstPreview.detail"), ...diagnostics);
  }
  lines.push(tr("typstPreview.hint"));
  if (templateContent.includes("\\n")) {
    lines.push(tr("typstPreview.hintLiteralNewline"));
    lines.push(tr("typstPreview.hintSetLetAfterNewline"));
    lines.push(tr("typstPreview.hintResetForm"));
  } else {
    lines.push(tr("typstPreview.hintCheckSetLet"));
    lines.push(tr("typstPreview.hintCheckSeparator"));
    lines.push(tr("typstPreview.hintResetForm"));
  }
  return lines.join("\n");
}

function buildResourceMissingMessage(rawMessage: string): string {
  const lines = [tr("typstPreview.resourceMissing")];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push(tr("typstPreview.detail"), ...diagnostics);
  }
  lines.push(tr("typstPreview.hint"));
  lines.push(tr("typstPreview.hintCheckImport"));
  lines.push(tr("typstPreview.hintOfficialPackage"));
  lines.push(tr("typstPreview.hintFontMissing"));
  return lines.join("\n");
}

function buildCompileFailedMessage(rawMessage: string): string {
  const lines = [tr("typstPreview.compileFailed")];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push(tr("typstPreview.detail"), ...diagnostics);
  }
  lines.push(tr("typstPreview.hintCheckResource"));
  return lines.join("\n");
}

export function formatTypstPreviewError(rawMessage: string, templateContent: string): string {
  const message = rawMessage.trim();
  const code = extractRenderCode(message);
  if (!code) {
    return message;
  }

  if (code === "RENDER_TEMPLATE_SYNTAX" || code === "RENDER_TEMPLATE_SEMANTIC") {
    return buildTemplateErrorMessage(message, templateContent);
  }
  if (code === "RENDER_RESOURCE_MISSING") {
    return buildResourceMissingMessage(message);
  }
  if (code === "RENDER_COMPILE_FAILED") {
    return buildCompileFailedMessage(message);
  }

  return message;
}

function tr(key: string) {
  return translate(getCurrentLocale(), key);
}
