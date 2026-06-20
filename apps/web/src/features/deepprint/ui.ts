import type { PrinterInfo } from "./types";
import { getCurrentLocale, translate } from "@/i18n";

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
  if (!printer.enabled) return translate(getCurrentLocale(), "printer.state.disabled");
  const normalized = (printer.state ?? "").toLowerCase();
  if (normalized.includes("idle")) return translate(getCurrentLocale(), "printer.state.idle");
  if (normalized.includes("process") || normalized.includes("busy")) return translate(getCurrentLocale(), "printer.state.processing");
  if (normalized.includes("stop")) return translate(getCurrentLocale(), "printer.state.stopped");
  return printer.state?.trim() || translate(getCurrentLocale(), "common.unknown");
}

export function printerSourceLabel(source: PrinterInfo["source"]) {
  switch (source) {
    case "manual":
      return translate(getCurrentLocale(), "printer.source.manual");
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
