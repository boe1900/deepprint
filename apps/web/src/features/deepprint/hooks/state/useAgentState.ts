import { useState } from "react";
import type { DeepHealthResponse, HealthResponse, PrinterInfo } from "../../types";

export function useAgentState() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [deepHealth, setDeepHealth] = useState<DeepHealthResponse | null>(null);
  const [printers, setPrinters] = useState<PrinterInfo[]>([]);
  const [printersNote, setPrintersNote] = useState("-");

  const [loadingHealth, setLoadingHealth] = useState(false);
  const [loadingPrinters, setLoadingPrinters] = useState(false);
  const [refreshingAll, setRefreshingAll] = useState(false);

  return {
    health,
    setHealth,
    deepHealth,
    setDeepHealth,
    printers,
    setPrinters,
    printersNote,
    setPrintersNote,
    loadingHealth,
    setLoadingHealth,
    loadingPrinters,
    setLoadingPrinters,
    refreshingAll,
    setRefreshingAll,
  };
}
