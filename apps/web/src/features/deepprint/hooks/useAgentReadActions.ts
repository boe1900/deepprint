import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError } from "../api";
import { AUTO_REFRESH_INTERVAL_MS } from "../constants";
import {
  createDeepHealthQueryOptions,
  createHealthQueryOptions,
  createPrintersQueryOptions,
} from "../queries";
import type {
  DeepHealthResponse,
  HealthResponse,
  NoticeState,
  PrinterInfo,
  RequestTimeoutSettings,
} from "../types";

interface UseAgentReadActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  setupLoading: boolean;
  showOnboarding: boolean;
  autoRefresh: boolean;
  setHealth: (value: HealthResponse | null) => void;
  setDeepHealth: (value: DeepHealthResponse | null) => void;
  setPrinters: (value: PrinterInfo[]) => void;
  setPrintersNote: (value: string) => void;
  setLoadingHealth: (value: boolean) => void;
  setLoadingPrinters: (value: boolean) => void;
  setRefreshingAll: (value: boolean) => void;
  setLastRefreshAt: (value: Date | null) => void;
  setNotice: (notice: NoticeState) => void;
}

interface UseAgentReadActionsResult {
  loadPrinters: () => Promise<void>;
  refreshAll: (silent: boolean) => Promise<void>;
}

export function useAgentReadActions({
  baseUrl,
  requestTimeouts,
  setupLoading,
  showOnboarding,
  autoRefresh,
  setHealth,
  setDeepHealth,
  setPrinters,
  setPrintersNote,
  setLoadingHealth,
  setLoadingPrinters,
  setRefreshingAll,
  setLastRefreshAt,
  setNotice,
}: UseAgentReadActionsArgs): UseAgentReadActionsResult {
  const queryClient = useQueryClient();
  const queriesEnabled = !setupLoading && !showOnboarding;
  const refetchInterval = queriesEnabled && autoRefresh ? AUTO_REFRESH_INTERVAL_MS : false;

  const healthQuery = useQuery({
    ...createHealthQueryOptions(baseUrl, requestTimeouts.health),
    enabled: queriesEnabled,
    refetchInterval,
  });

  const deepHealthQuery = useQuery({
    ...createDeepHealthQueryOptions(baseUrl, requestTimeouts.deepHealth),
    enabled: queriesEnabled,
    refetchInterval,
  });

  const printersQuery = useQuery({
    ...createPrintersQueryOptions(baseUrl, requestTimeouts.printers),
    enabled: queriesEnabled,
    refetchInterval,
  });

  useEffect(() => {
    if (healthQuery.data) {
      setHealth(healthQuery.data);
    }
  }, [healthQuery.data, setHealth]);

  useEffect(() => {
    if (deepHealthQuery.data) {
      setDeepHealth(deepHealthQuery.data);
    }
  }, [deepHealthQuery.data, setDeepHealth]);

  useEffect(() => {
    if (printersQuery.data) {
      setPrinters(printersQuery.data.printers);
      setPrintersNote(printersQuery.data.note ?? tr("printers.loadedManaged"));
    }
  }, [printersQuery.data, setPrinters, setPrintersNote]);

  useEffect(() => {
    setLoadingHealth(queriesEnabled && (healthQuery.isFetching || deepHealthQuery.isFetching));
  }, [
    deepHealthQuery.isFetching,
    healthQuery.isFetching,
    queriesEnabled,
    setLoadingHealth,
  ]);

  useEffect(() => {
    setLoadingPrinters(queriesEnabled && printersQuery.isFetching);
  }, [printersQuery.isFetching, queriesEnabled, setLoadingPrinters]);

  useEffect(() => {
    if (!healthQuery.data || !deepHealthQuery.data || !printersQuery.data) return;
    const updatedAt = Math.max(
      healthQuery.dataUpdatedAt,
      deepHealthQuery.dataUpdatedAt,
      printersQuery.dataUpdatedAt,
    );
    if (updatedAt > 0) {
      setLastRefreshAt(new Date(updatedAt));
    }
  }, [
    deepHealthQuery.data,
    deepHealthQuery.dataUpdatedAt,
    healthQuery.data,
    healthQuery.dataUpdatedAt,
    printersQuery.data,
    printersQuery.dataUpdatedAt,
    setLastRefreshAt,
  ]);

  const loadPrinters = useCallback(async () => {
    setLoadingPrinters(true);

    try {
      const data = await queryClient.fetchQuery({
        ...createPrintersQueryOptions(baseUrl, requestTimeouts.printers),
        staleTime: 0,
      });

      setPrinters(data.printers);
      setPrintersNote(data.note ?? tr("printers.loadedManaged"));
    } catch (error) {
      if (isAbortError(error)) return;

      const message = getRequestErrorMessage(error, tr("printers.refreshFailed"));
      setNotice({ kind: "error", message });
    } finally {
      setLoadingPrinters(false);
    }
  }, [
    baseUrl,
    queryClient,
    requestTimeouts.printers,
    setLoadingPrinters,
    setNotice,
    setPrinters,
    setPrintersNote,
  ]);

  const refreshAll = useCallback(
    async (silent: boolean) => {
      if (!silent) setRefreshingAll(true);
      setLoadingHealth(true);
      setLoadingPrinters(true);

      try {
        const [healthData, deepData, printersData] = await Promise.all([
          queryClient.fetchQuery({
            ...createHealthQueryOptions(baseUrl, requestTimeouts.health),
            staleTime: 0,
          }),
          queryClient.fetchQuery({
            ...createDeepHealthQueryOptions(baseUrl, requestTimeouts.deepHealth),
            staleTime: 0,
          }),
          queryClient.fetchQuery({
            ...createPrintersQueryOptions(baseUrl, requestTimeouts.printers),
            staleTime: 0,
          }),
        ]);

        setHealth(healthData);
        setDeepHealth(deepData);
        setPrinters(printersData.printers);
        setPrintersNote(printersData.note ?? tr("printers.loadedManaged"));
        setLastRefreshAt(new Date());

        if (!silent) {
          setNotice({ kind: "ok", message: tr("common.refreshComplete") });
        }
      } catch (error) {
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("printers.refreshServiceFailed"));
        setNotice({ kind: "error", message });
      } finally {
        setLoadingHealth(false);
        setLoadingPrinters(false);
        if (!silent) setRefreshingAll(false);
      }
    },
    [
      baseUrl,
      queryClient,
      requestTimeouts.deepHealth,
      requestTimeouts.health,
      requestTimeouts.printers,
      setDeepHealth,
      setHealth,
      setLastRefreshAt,
      setLoadingHealth,
      setLoadingPrinters,
      setNotice,
      setPrinters,
      setPrintersNote,
      setRefreshingAll,
    ],
  );

  return {
    loadPrinters,
    refreshAll,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
