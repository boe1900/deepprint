import { useEffect, useState } from "react";
import {
  BASE_URL_STORAGE_KEY,
  DEFAULT_BASE_URL,
  REQUEST_TIMEOUTS_MS,
  REQUEST_TIMEOUTS_STORAGE_KEY,
  THEME_MODE_STORAGE_KEY,
} from "../../constants";
import { normalizeBaseUrl } from "../../utils";
import type {
  BaseUrlProbeResult,
  NoticeState,
  RequestTimeoutSettings,
  ThemeMode,
} from "../../types";

function normalizeThemeMode(value: string | null): ThemeMode {
  if (value === "dark" || value === "light" || value === "system") return value;
  return "system";
}

function clampTimeoutMs(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  const next = Math.round(parsed);
  if (next < 1000) return 1000;
  if (next > 120000) return 120000;
  return next;
}

function normalizeRequestTimeouts(raw: unknown): RequestTimeoutSettings {
  if (!raw || typeof raw !== "object") {
    return { ...REQUEST_TIMEOUTS_MS };
  }

  const data = raw as Partial<Record<keyof RequestTimeoutSettings, unknown>>;
  return {
    health: clampTimeoutMs(data.health, REQUEST_TIMEOUTS_MS.health),
    deepHealth: clampTimeoutMs(data.deepHealth, REQUEST_TIMEOUTS_MS.deepHealth),
    printers: clampTimeoutMs(data.printers, REQUEST_TIMEOUTS_MS.printers),
    jobStatus: clampTimeoutMs(data.jobStatus, REQUEST_TIMEOUTS_MS.jobStatus),
    writes: clampTimeoutMs(data.writes, REQUEST_TIMEOUTS_MS.writes),
    diagnosticsExport: clampTimeoutMs(
      data.diagnosticsExport,
      REQUEST_TIMEOUTS_MS.diagnosticsExport,
    ),
    urlProbe: clampTimeoutMs(data.urlProbe, REQUEST_TIMEOUTS_MS.urlProbe),
  };
}

export function useUiState() {
  const [baseUrl, setBaseUrl] = useState<string>(() => {
    const cached = window.localStorage.getItem(BASE_URL_STORAGE_KEY);
    return cached ? normalizeBaseUrl(cached) : DEFAULT_BASE_URL;
  });
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [lastRefreshAt, setLastRefreshAt] = useState<Date | null>(null);
  const [themeMode, setThemeMode] = useState<ThemeMode>(() =>
    normalizeThemeMode(window.localStorage.getItem(THEME_MODE_STORAGE_KEY)),
  );
  const [baseUrlProbe, setBaseUrlProbe] = useState<BaseUrlProbeResult | null>(null);
  const [requestTimeouts, setRequestTimeouts] = useState<RequestTimeoutSettings>(() => {
    try {
      const cached = window.localStorage.getItem(REQUEST_TIMEOUTS_STORAGE_KEY);
      if (!cached) return { ...REQUEST_TIMEOUTS_MS };
      return normalizeRequestTimeouts(JSON.parse(cached) as unknown);
    } catch {
      return { ...REQUEST_TIMEOUTS_MS };
    }
  });

  useEffect(() => {
    if (!notice || notice.kind !== "ok") return undefined;
    const timer = window.setTimeout(() => {
      setNotice(null);
    }, 4000);
    return () => window.clearTimeout(timer);
  }, [notice]);

  return {
    baseUrl,
    setBaseUrl,
    notice,
    setNotice,
    autoRefresh,
    setAutoRefresh,
    lastRefreshAt,
    setLastRefreshAt,
    themeMode,
    setThemeMode,
    baseUrlProbe,
    setBaseUrlProbe,
    requestTimeouts,
    setRequestTimeouts,
  };
}
