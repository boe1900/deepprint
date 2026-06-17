import { useCallback, useEffect, useRef } from "react";
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
        const message = getRequestErrorMessage(error, "连通性预检失败");
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
      message: "检查中...",
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
        setNotice({ kind: "error", message: `连通性预检失败: ${result.message}` });
        return;
      }

      setOpsProbe({
        status: "ok",
        message: result.message,
        latency_ms: result.latency_ms || latency,
        checked_at_ms: Date.now(),
      });
      setNotice({ kind: "ok", message: "连通性预检通过" });
    } catch (error) {
      if (requestId !== opsProbeRequestIdRef.current) return;

      if (isAbortError(error)) {
        setOpsProbe({
          status: "idle",
          message: "已取消",
          latency_ms: null,
          checked_at_ms: null,
        });
        return;
      }

      const message = getRequestErrorMessage(error, "连通性预检失败");
      const latency = Math.round(performance.now() - startedAt);
      setOpsProbe({
        status: "error",
        message,
        latency_ms: latency,
        checked_at_ms: Date.now(),
      });
      setNotice({ kind: "error", message: `连通性预检失败: ${message}` });
    }
  }, [baseUrl, onProbeBaseUrl, setNotice, setOpsProbe]);

  const onCopyDiagnosticPath = useCallback(
    async (bundlePath: string) => {
      try {
        if (!navigator.clipboard?.writeText) {
          throw new Error("当前环境不支持剪贴板写入");
        }
        await navigator.clipboard.writeText(bundlePath);
        setNotice({ kind: "ok", message: "已复制诊断包路径" });
      } catch (error) {
        const message = error instanceof Error ? error.message : "复制失败";
        setNotice({ kind: "error", message: `复制失败: ${message}` });
      }
    },
    [setNotice],
  );

  const onClearDiagnosticsHistory = useCallback(() => {
    const target = normalizeBaseUrl(baseUrl);
    setDiagHistory((previous) => previous.filter((item) => item.base_url !== target));
    setNotice({ kind: "ok", message: "已清理当前 Agent 的诊断历史" });
  }, [baseUrl, setDiagHistory, setNotice]);

  const onClearAllDiagnosticsHistory = useCallback(() => {
    setDiagHistory(() => []);
    setNotice({ kind: "ok", message: "已清空全部诊断历史" });
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
      setNotice({ kind: "ok", message: "已删除诊断历史记录" });
    },
    [setDiagHistory, setNotice],
  );

  const onResetBaseUrl = useCallback(() => {
    setBaseUrl(DEFAULT_BASE_URL);
    setBaseUrlProbe(null);
    setNotice({ kind: "ok", message: "已恢复默认 Agent URL" });
  }, [setBaseUrl, setBaseUrlProbe, setNotice]);

  const onResetViewSettings = useCallback(() => {
    setAutoRefresh(true);
    setJobAutoPoll(true);
    setJobPollIntervalSec(DEFAULT_JOB_POLL_INTERVAL_SEC);
    setNotice({ kind: "ok", message: "已恢复默认轮询设置" });
  }, [setAutoRefresh, setJobAutoPoll, setJobPollIntervalSec, setNotice]);

  const onResetRequestTimeouts = useCallback(() => {
    setRequestTimeouts({ ...REQUEST_TIMEOUTS_MS });
    setNotice({ kind: "ok", message: "已恢复默认请求超时" });
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
