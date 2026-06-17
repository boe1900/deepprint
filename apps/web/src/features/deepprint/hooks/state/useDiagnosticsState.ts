import { useState } from "react";
import { loadDiagnosticsHistory } from "../../utils";
import type { DiagnosticExportResponse, DiagnosticHistoryItem, OpsProbeState } from "../../types";

export function useDiagnosticsState() {
  const [diagLoading, setDiagLoading] = useState(false);
  const [diagResult, setDiagResult] = useState<DiagnosticExportResponse | null>(null);
  const [diagHistory, setDiagHistory] = useState<DiagnosticHistoryItem[]>(() =>
    loadDiagnosticsHistory(),
  );
  const [opsProbe, setOpsProbe] = useState<OpsProbeState>({
    status: "idle",
    message: "-",
    latency_ms: null,
    checked_at_ms: null,
  });

  return {
    diagLoading,
    setDiagLoading,
    diagResult,
    setDiagResult,
    diagHistory,
    setDiagHistory,
    opsProbe,
    setOpsProbe,
  };
}
