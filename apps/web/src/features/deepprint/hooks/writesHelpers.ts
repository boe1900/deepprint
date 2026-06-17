export interface TemplatePrintOptionsInput {
  copies: string;
  paperSize: string;
  duplex: string;
}

export function buildTemplatePrintOptions(
  input: TemplatePrintOptionsInput,
): Record<string, unknown> {
  const printOptions: Record<string, unknown> = {};

  const copiesRaw = input.copies.trim();
  if (copiesRaw) {
    const copies = Number.parseInt(copiesRaw, 10);
    if (!Number.isFinite(copies) || copies < 1 || copies > 100) {
      throw new Error("copies 必须为 1-100 的整数");
    }
    printOptions.copies = copies;
  }

  const paperSize = input.paperSize.trim();
  if (paperSize) printOptions.media = paperSize;

  const duplex = input.duplex.trim();
  if (duplex) printOptions.sides = duplex;

  return printOptions;
}

export function parseTemplateDataJson(createDataJson: string): unknown {
  try {
    return JSON.parse(createDataJson);
  } catch {
    throw new Error("数据 JSON 解析失败，请检查格式");
  }
}

export function resolveTemplateContent(createTemplateContent: string): string {
  const templateContent = createTemplateContent.trim();
  if (!templateContent) {
    throw new Error("模板内容不能为空");
  }
  return templateContent;
}

export function readPreviewRequiredHeader(headers: Headers, name: string): string {
  const value = headers.get(name)?.trim();
  if (!value) {
    throw new Error(`预览响应缺少字段：${name}`);
  }
  return value;
}

export function parsePreviewRequiredNumber(headers: Headers, name: string): number {
  const value = readPreviewRequiredHeader(headers, name);
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(`预览响应字段无效：${name}=${value}`);
  }
  return parsed;
}

export function parsePreviewOptionalNumber(headers: Headers, name: string): number | null {
  const value = headers.get(name)?.trim();
  if (!value) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(`预览响应字段无效：${name}=${value}`);
  }
  return parsed;
}
