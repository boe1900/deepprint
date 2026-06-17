import type { PrinterInfo } from "./types";

export function statusBadgeVariant(status: string | null | undefined) {
  const normalized = (status ?? "").toLowerCase();
  if (normalized.includes("idle") || normalized === "ok" || normalized === "succeeded") {
    return "secondary" as const;
  }
  if (
    normalized.includes("failed") ||
    normalized.includes("error") ||
    normalized.includes("stop") ||
    normalized.includes("canceled")
  ) {
    return "destructive" as const;
  }
  return "outline" as const;
}

export function noticeBadgeVariant(kind: "ok" | "error") {
  return kind === "ok" ? ("secondary" as const) : ("destructive" as const);
}

export function printerStateLabel(printer: PrinterInfo) {
  if (!printer.enabled) return "已停用";
  const normalized = (printer.state ?? "").toLowerCase();
  if (normalized.includes("idle")) return "空闲";
  if (normalized.includes("process") || normalized.includes("busy")) return "处理中";
  if (normalized.includes("stop")) return "已停止";
  return printer.state?.trim() || "未知";
}

export function printerSourceLabel(source: PrinterInfo["source"]) {
  switch (source) {
    case "manual":
      return "手动";
    case "cups_import":
      return "CUPS";
    case "mdns":
      return "mDNS";
    default:
      return source;
  }
}

export function selectDefaultPrinter(printers: PrinterInfo[]) {
  return printers.find((printer) => printer.is_default)?.id ?? printers[0]?.id ?? "";
}
