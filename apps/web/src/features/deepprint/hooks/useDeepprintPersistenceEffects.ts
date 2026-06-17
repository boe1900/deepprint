import { useEffect } from "react";
import {
  BASE_URL_STORAGE_KEY,
  DIAGNOSTICS_HISTORY_MAX_ITEMS,
  DIAGNOSTICS_HISTORY_STORAGE_KEY,
  REQUEST_TIMEOUTS_STORAGE_KEY,
  THEME_MODE_STORAGE_KEY,
} from "../constants";
import { normalizeBaseUrl } from "../utils";
import type { DiagnosticHistoryItem, RequestTimeoutSettings, ThemeMode } from "../types";

interface UseDeepprintPersistenceEffectsArgs {
  baseUrl: string;
  diagHistory: DiagnosticHistoryItem[];
  previewPdfUrl: string | null;
  themeMode: ThemeMode;
  requestTimeouts: RequestTimeoutSettings;
}

export function useDeepprintPersistenceEffects({
  baseUrl,
  diagHistory,
  previewPdfUrl,
  themeMode,
  requestTimeouts,
}: UseDeepprintPersistenceEffectsArgs) {
  useEffect(() => {
    window.localStorage.setItem(BASE_URL_STORAGE_KEY, normalizeBaseUrl(baseUrl));
  }, [baseUrl]);

  useEffect(() => {
    const root = document.documentElement;
    const resolveTheme = (): "dark" | "light" => {
      if (themeMode === "dark" || themeMode === "light") return themeMode;
      return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    };

    const applyTheme = () => {
      const resolvedTheme = resolveTheme();
      root.dataset.themeMode = themeMode;
      root.dataset.theme = resolvedTheme;
      root.classList.toggle("dark", resolvedTheme === "dark");
      window.localStorage.setItem(THEME_MODE_STORAGE_KEY, themeMode);
    };

    applyTheme();

    if (themeMode !== "system") return undefined;

    const media = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => applyTheme();

    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", onChange);
      return () => media.removeEventListener("change", onChange);
    }

    media.addListener(onChange);
    return () => media.removeListener(onChange);
  }, [themeMode]);

  useEffect(() => {
    window.localStorage.setItem(
      DIAGNOSTICS_HISTORY_STORAGE_KEY,
      JSON.stringify(diagHistory.slice(0, DIAGNOSTICS_HISTORY_MAX_ITEMS)),
    );
  }, [diagHistory]);

  useEffect(() => {
    window.localStorage.setItem(REQUEST_TIMEOUTS_STORAGE_KEY, JSON.stringify(requestTimeouts));
  }, [requestTimeouts]);

  useEffect(
    () => () => {
      if (previewPdfUrl) {
        URL.revokeObjectURL(previewPdfUrl);
      }
    },
    [previewPdfUrl],
  );
}
