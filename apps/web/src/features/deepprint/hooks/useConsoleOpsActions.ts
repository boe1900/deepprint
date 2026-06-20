import { useCallback, useEffect, useRef } from "react";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError, requestJson } from "../api";
import { DEFAULT_BASE_URL, DEFAULT_JOB_POLL_INTERVAL_SEC, REQUEST_TIMEOUTS_MS } from "../constants";
import { normalizeBaseUrl } from "../utils";
import type {
  BaseUrlProbeResult,
  DiagnosticHistoryItem,
  HealthResponse,
  NoticeState,
  OpsProbeState,
  RequestTimeoutSettings,
} from "../types";

interface UseConsoleOpsActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  setOpsProbe: (value: OpsProbeState) => void;
  setNotice: (notice: NoticeState) => void;
  setBaseUrlProbe: (value: BaseUrlProbeResult | null) => void;
  setDiagHistory: (updater: (previous: DiagnosticHistoryItem[]) => DiagnosticHistoryItem[]) => void;
  setBaseUrl: (value: string) => void;
  setAutoRefresh: (value: boolean) => void;
  setJobAutoPoll: (value: boolean) => void;
  setJobPollIntervalSec: (value: number) => void;
  setRequestTimeouts: (value: RequestTimeoutSettings) => void;
}

interface UseConsoleOpsActionsResult {
  onProbeBaseUrl: (candidateBaseUrl: string) => Promise<BaseUrlProbeResult>;
  onRunOpsProbe: () => Promise<void>;
  onCopyDiagnosticPath: (bundlePath: string) => Promise<void>;
  onClearDiagnosticsHistory: () => void;
  onClearAllDiagnosticsHistory: () => void;
  onDeleteDiagnosticHistoryItem: (item: DiagnosticHistoryItem) => void;
  onResetBaseUrl: () => void;
  onResetViewSettings: () => void;
  onResetRequestTimeouts: () => void;
}

export function useConsoleOpsActions({
  baseUrl,
  requestTimeouts,
  setOpsProbe,
  setNotice,
  setBaseUrlProbe,
  setDiagHistory,
  setBaseUrl,
  setAutoRefresh,
  setJobAutoPoll,
  setJobPollIntervalSec,
  setRequestTimeouts,
}: UseConsoleOpsActionsArgs): UseConsoleOpsActionsResult {
  const probeAbortRef = useRef<AbortController | null>(null);
  const opsProbeRequestIdRef = useRef(0);

  const onProbeBaseUrl = useCallback(
    async (candidateBaseUrl: string) => {
      const normalizedBaseUrl = normalizeBaseUrl(candidateBaseUrl);
      probeAbortRef.current?.abort();
      const controller = new AbortController();
      probeAbortRef.current = controller;

      const startedAt = performance.now();
      try {
        const data = await requestJson<HealthResponse>(normalizedBaseUrl, "/v1/health", {
          signal: controller.signal,
          timeoutMs: requestTimeouts.urlProbe,
        });
        const latency = Math.round(performance.now() - startedAt);
        const result: BaseUrlProbeResult = {
          ok: true,
          message: `${data.status} / v${data.version}`,
          latency_ms: latency,
          normalized_base_url: normalizedBaseUrl,
          checked_at_ms: Date.now(),
        };
        setBaseUrlProbe(result);
        return result;
      } catch (error) {
        if (isAbortError(error)) {
          throw error;
        }

        const latency = Math.round(performance.now() - startedAt);
        const message = getRequestErrorMessage(error, tr("ops.probeFailed"));
        const result: BaseUrlProbeResult = {
          ok: false,
          message,
          latency_ms: latency,
          normalized_base_url: normalizedBaseUrl,
          checked_at_ms: Date.now(),
        };
        setBaseUrlProbe(result);
        return result;
      } finally {
        if (probeAbortRef.current === controller) {
          probeAbortRef.current = null;
        }
      }
    },
    [requestTimeouts.urlProbe, setBaseUrlProbe],
  );

  const onRunOpsProbe = useCallback(async () => {
    const requestId = ++opsProbeRequestIdRef.current;
    setOpsProbe({
      status: "checking",
      message: tr("ops.checking"),
      latency_ms: null,
      checked_at_ms: null,
    });

    const startedAt = performance.now();
    try {
      const result = await onProbeBaseUrl(baseUrl);
      if (requestId !== opsProbeRequestIdRef.current) return;
      const latency = Math.round(performance.now() - startedAt);
      if (!result.ok) {
        setOpsProbe({
          status: "error",
          message: result.message,
          latency_ms: result.latency_ms || latency,
          checked_at_ms: Date.now(),
        });
        setNotice({ kind: "error", message: tr("ops.probeFailedWithMessage", { message: result.message }) });
        return;
      }

      setOpsProbe({
        status: "ok",
        message: result.message,
        latency_ms: result.latency_ms || latency,
        checked_at_ms: Date.now(),
      });
      setNotice({ kind: "ok", message: tr("ops.probePassed") });
    } catch (error) {
      if (requestId !== opsProbeRequestIdRef.current) return;

      if (isAbortError(error)) {
        setOpsProbe({
          status: "idle",
          message: tr("ops.cancelled"),
          latency_ms: null,
          checked_at_ms: null,
        });
        return;
      }

      const message = getRequestErrorMessage(error, tr("ops.probeFailed"));
      const latency = Math.round(performance.now() - startedAt);
      setOpsProbe({
        status: "error",
        message,
        latency_ms: latency,
        checked_at_ms: Date.now(),
      });
      setNotice({ kind: "error", message: tr("ops.probeFailedWithMessage", { message }) });
    }
  }, [baseUrl, onProbeBaseUrl, setNotice, setOpsProbe]);

  const onCopyDiagnosticPath = useCallback(
    async (bundlePath: string) => {
      try {
        if (!navigator.clipboard?.writeText) {
          throw new Error(tr("ops.clipboardUnsupported"));
        }
        await navigator.clipboard.writeText(bundlePath);
        setNotice({ kind: "ok", message: tr("ops.copyDiagnosticPathNotice") });
      } catch (error) {
        const message = error instanceof Error ? error.message : tr("ops.copyDiagnosticFailed");
        setNotice({ kind: "error", message: tr("ops.copyDiagnosticFailedWithMessage", { message }) });
      }
    },
    [setNotice],
  );

  const onClearDiagnosticsHistory = useCallback(() => {
    const target = normalizeBaseUrl(baseUrl);
    setDiagHistory((previous) => previous.filter((item) => item.base_url !== target));
    setNotice({ kind: "ok", message: tr("ops.clearCurrentDiagnosticsHistory") });
  }, [baseUrl, setDiagHistory, setNotice]);

  const onClearAllDiagnosticsHistory = useCallback(() => {
    setDiagHistory(() => []);
    setNotice({ kind: "ok", message: tr("ops.clearAllDiagnosticsHistory") });
  }, [setDiagHistory, setNotice]);

  const onDeleteDiagnosticHistoryItem = useCallback(
    (targetItem: DiagnosticHistoryItem) => {
      setDiagHistory((previous) =>
        previous.filter(
          (item) =>
            !(
              item.bundle_id === targetItem.bundle_id &&
              item.base_url === targetItem.base_url &&
              item.exported_at_ms === targetItem.exported_at_ms
            ),
        ),
      );
      setNotice({ kind: "ok", message: tr("ops.deleteDiagnosticHistoryNotice") });
    },
    [setDiagHistory, setNotice],
  );

  const onResetBaseUrl = useCallback(() => {
    setBaseUrl(DEFAULT_BASE_URL);
    setBaseUrlProbe(null);
    setNotice({ kind: "ok", message: tr("ops.resetBaseUrl") });
  }, [setBaseUrl, setBaseUrlProbe, setNotice]);

  const onResetViewSettings = useCallback(() => {
    setAutoRefresh(true);
    setJobAutoPoll(true);
    setJobPollIntervalSec(DEFAULT_JOB_POLL_INTERVAL_SEC);
    setNotice({ kind: "ok", message: tr("ops.resetViewSettings") });
  }, [setAutoRefresh, setJobAutoPoll, setJobPollIntervalSec, setNotice]);

  const onResetRequestTimeouts = useCallback(() => {
    setRequestTimeouts({ ...REQUEST_TIMEOUTS_MS });
    setNotice({ kind: "ok", message: tr("ops.resetRequestTimeouts") });
  }, [setNotice, setRequestTimeouts]);

  useEffect(
    () => () => {
      probeAbortRef.current?.abort();
      opsProbeRequestIdRef.current += 1;
    },
    [],
  );

  return {
    onProbeBaseUrl,
    onRunOpsProbe,
    onCopyDiagnosticPath,
    onClearDiagnosticsHistory,
    onClearAllDiagnosticsHistory,
    onDeleteDiagnosticHistoryItem,
    onResetBaseUrl,
    onResetViewSettings,
    onResetRequestTimeouts,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
