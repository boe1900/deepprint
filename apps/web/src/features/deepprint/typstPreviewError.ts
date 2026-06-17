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
  const lines = ["模板格式有误，无法生成预览。"];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push("错误详情：", ...diagnostics);
  }
  lines.push("建议：");
  if (templateContent.includes("\\n")) {
    lines.push("1. 检测到模板中包含字面量 \\n，请改为真实换行。");
    lines.push("2. 检查 #set/#let 语句后是否换行，避免同一行拼接多个语句。");
    lines.push("3. 可先点击“重置表单”恢复默认模板再逐步修改。");
  } else {
    lines.push("1. 检查 #set/#let 语句后是否换行，避免同一行拼接多个语句。");
    lines.push("2. 检查文本与表达式之间的分隔符是否完整。");
    lines.push("3. 可先点击“重置表单”恢复默认模板再逐步修改。");
  }
  return lines.join("\n");
}

function buildResourceMissingMessage(rawMessage: string): string {
  const lines = ["模板依赖的资源缺失，无法生成预览。"];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push("错误详情：", ...diagnostics);
  }
  lines.push("建议：");
  lines.push("1. 检查 #import 的包名和版本是否正确。");
  lines.push("2. 若是官方包，先在设置页“Typst 包管理”刷新/安装。");
  lines.push("3. 若是字体缺失，先在设置页“Typst 字体管理”安装字体。");
  return lines.join("\n");
}

function buildCompileFailedMessage(rawMessage: string): string {
  const lines = ["Typst 编译失败，无法生成预览。"];
  const diagnostics = formatDiagnosticsLines(rawMessage);
  if (diagnostics.length > 0) {
    lines.push("错误详情：", ...diagnostics);
  }
  lines.push("建议：请先检查模板语法和依赖资源是否完整。");
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
