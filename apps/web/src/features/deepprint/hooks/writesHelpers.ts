import { getCurrentLocale, translate } from "@/i18n";

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
      throw new Error(tr("writes.copiesInvalid"));
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
    throw new Error(tr("writes.dataJsonInvalid"));
  }
}

export function resolveTemplateContent(createTemplateContent: string): string {
  const templateContent = createTemplateContent.trim();
  if (!templateContent) {
    throw new Error(tr("writes.templateContentRequired"));
  }
  return templateContent;
}

export function readPreviewRequiredHeader(headers: Headers, name: string): string {
  const value = headers.get(name)?.trim();
  if (!value) {
    throw new Error(tr("writes.previewHeaderMissing", { name }));
  }
  return value;
}

export function parsePreviewRequiredNumber(headers: Headers, name: string): number {
  const value = readPreviewRequiredHeader(headers, name);
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(tr("writes.previewHeaderInvalid", { name, value }));
  }
  return parsed;
}

export function parsePreviewOptionalNumber(headers: Headers, name: string): number | null {
  const value = headers.get(name)?.trim();
  if (!value) return null;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    throw new Error(tr("writes.previewHeaderInvalid", { name, value }));
  }
  return parsed;
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
