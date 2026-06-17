import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getRequestErrorMessage, requestJson, signClientWriteHeaders, isAbortError } from "../api";
import {
  createTypstPackagesQueryOptions,
  deepprintQueryKeys,
} from "../queries";
import { useLatestAsyncTask } from "./useLatestAsyncTask";
import { readFileAsBase64 } from "../utils";
import type {
  ClearTypstPreviewCacheResponse,
  DeleteTypstPackageResponse,
  InstallTypstPackageResponse,
  NoticeState,
  RequestTimeoutSettings,
  TypstPackageInfo,
  TypstPackagesResponse,
} from "../types";

interface UseTypstPackagesActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  authRequiredForWrites: boolean;
  writeAuthToken: string;
  writeAuthSecret: string;
  setNotice: (notice: NoticeState) => void;
  setPackages: (value: TypstPackageInfo[]) => void;
  setLoading: (value: boolean) => void;
  setError: (value: string | null) => void;
  setInstalling: (value: boolean) => void;
  setDeletingKey: (value: string | null) => void;
  setClearingPreviewCache: (value: boolean) => void;
}

interface UseTypstPackagesActionsResult {
  loadTypstPackages: (silent?: boolean) => Promise<void>;
  onInstallTypstPackage: (file: File) => Promise<void>;
  onDeleteTypstPackage: (pkg: TypstPackageInfo) => Promise<void>;
  onClearTypstPreviewCache: () => Promise<void>;
}

export function useTypstPackagesActions({
  baseUrl,
  requestTimeouts,
  authRequiredForWrites,
  writeAuthToken,
  writeAuthSecret,
  setNotice,
  setPackages,
  setLoading,
  setError,
  setInstalling,
  setDeletingKey,
  setClearingPreviewCache,
}: UseTypstPackagesActionsArgs): UseTypstPackagesActionsResult {
  const queryClient = useQueryClient();
  const installTask = useLatestAsyncTask();
  const deleteTask = useLatestAsyncTask();
  const clearTask = useLatestAsyncTask();
  const packagesQuery = useQuery({
    ...createTypstPackagesQueryOptions(baseUrl, requestTimeouts.writes),
  });

  useEffect(() => {
    if (!packagesQuery.data) return;
    setPackages(packagesQuery.data.packages);
    setError(null);
  }, [packagesQuery.data, setError, setPackages]);

  useEffect(() => {
    setLoading(packagesQuery.isFetching);
  }, [packagesQuery.isFetching, setLoading]);

  const prepareWriteHeaders = useCallback(
    async (method: string, path: string, bodyText: string): Promise<Record<string, string>> => {
      const token = writeAuthToken.trim();
      const secret = writeAuthSecret.trim();
      const hasAnyAuthInput = token.length > 0 || secret.length > 0;

      if (!authRequiredForWrites && !hasAnyAuthInput) {
        return {};
      }

      if (hasAnyAuthInput && (!token || !secret)) {
        throw new Error("若要发送签名请求，请同时填写 token 与 secret");
      }

      try {
        return await signClientWriteHeaders(method, path, bodyText, token || null, secret || null);
      } catch (error) {
        const message = error instanceof Error ? error.message : "签名失败";
        if (authRequiredForWrites) {
          throw new Error(
            `当前 Agent 已启用操作鉴权，签名失败：${message}。请确认初始化向导已保存凭据，或在当前界面输入 token 与 secret。`,
          );
        }
        throw new Error(`签名失败：${message}`);
      }
    },
    [authRequiredForWrites, writeAuthSecret, writeAuthToken],
  );

  const loadTypstPackages = useCallback(
    async (silent = false) => {
      if (!silent) {
        setError(null);
      }

      try {
        await queryClient.fetchQuery({
          ...createTypstPackagesQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
      } catch (error) {
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, "加载 Typst 包失败");
        setError(message);
        if (!silent) {
          setNotice({ kind: "error", message: `加载 Typst 包失败: ${message}` });
        }
      }
    },
    [
      baseUrl,
      queryClient,
      requestTimeouts.writes,
      setError,
      setNotice,
    ],
  );

  const onInstallTypstPackage = useCallback(
    async (file: File) => {
      const ticket = installTask.start();
      setInstalling(true);
      setError(null);

      try {
        const fileContentBase64 = await readFileAsBase64(file);
        const path = "/v1/typst/packages/install";
        const bodyText = JSON.stringify({
          archive_base64: fileContentBase64,
          file_name: file.name,
          replace_existing: true,
        });
        const headers = await prepareWriteHeaders("POST", path, bodyText);
        const result = await requestJson<InstallTypstPackageResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: requestTimeouts.writes,
        });

        if (!ticket.isCurrent()) return;
        await queryClient.invalidateQueries({
          queryKey: deepprintQueryKeys.typstPackages(baseUrl),
        });
        await queryClient.fetchQuery({
          ...createTypstPackagesQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
        setNotice({
          kind: "ok",
          message: `已安装 Typst 包 @${result.namespace}/${result.name}:${result.version}`,
        });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, "安装 Typst 包失败");
        setError(message);
        setNotice({ kind: "error", message: `安装 Typst 包失败: ${message}` });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setInstalling(false);
        }
      }
    },
    [
      baseUrl,
      installTask,
      prepareWriteHeaders,
      queryClient,
      requestTimeouts.writes,
      setError,
      setInstalling,
      setNotice,
    ],
  );

  const onDeleteTypstPackage = useCallback(
    async (pkg: TypstPackageInfo) => {
      const ticket = deleteTask.start();
      const key = `${pkg.origin}:${pkg.namespace}/${pkg.name}:${pkg.version}`;
      setDeletingKey(key);

      try {
        const path = "/v1/typst/packages/delete";
        const bodyText = JSON.stringify({
          origin: pkg.origin,
          namespace: pkg.namespace,
          name: pkg.name,
          version: pkg.version,
        });
        const headers = await prepareWriteHeaders("POST", path, bodyText);
        await requestJson<DeleteTypstPackageResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: requestTimeouts.writes,
        });

        if (!ticket.isCurrent()) return;
        await queryClient.invalidateQueries({
          queryKey: deepprintQueryKeys.typstPackages(baseUrl),
        });
        await queryClient.fetchQuery({
          ...createTypstPackagesQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
        setNotice({
          kind: "ok",
          message: `已删除 Typst 包 @${pkg.namespace}/${pkg.name}:${pkg.version}`,
        });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, "删除 Typst 包失败");
        setError(message);
        setNotice({ kind: "error", message: `删除 Typst 包失败: ${message}` });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setDeletingKey(null);
        }
      }
    },
    [
      baseUrl,
      deleteTask,
      prepareWriteHeaders,
      queryClient,
      requestTimeouts.writes,
      setDeletingKey,
      setError,
      setNotice,
    ],
  );

  const onClearTypstPreviewCache = useCallback(async () => {
    const ticket = clearTask.start();
    setClearingPreviewCache(true);

    try {
      const path = "/v1/typst/packages/clear-preview-cache";
      const bodyText = JSON.stringify({});
      const headers = await prepareWriteHeaders("POST", path, bodyText);
      await requestJson<ClearTypstPreviewCacheResponse>(baseUrl, path, {
        method: "POST",
        body: bodyText,
        headers,
        signal: ticket.signal,
        timeoutMs: requestTimeouts.writes,
      });

      if (!ticket.isCurrent()) return;
      await queryClient.invalidateQueries({
        queryKey: deepprintQueryKeys.typstPackages(baseUrl),
      });
      await queryClient.fetchQuery({
        ...createTypstPackagesQueryOptions(baseUrl, requestTimeouts.writes),
        staleTime: 0,
      });
      setNotice({ kind: "ok", message: "已清理 preview 包缓存" });
    } catch (error) {
      if (!ticket.isCurrent()) return;
      if (isAbortError(error)) return;

      const message = getRequestErrorMessage(error, "清理 preview 缓存失败");
      setError(message);
      setNotice({ kind: "error", message: `清理 preview 缓存失败: ${message}` });
    } finally {
      ticket.finish();
      if (ticket.isCurrent()) {
        setClearingPreviewCache(false);
      }
    }
  }, [
    baseUrl,
    clearTask,
    prepareWriteHeaders,
    queryClient,
    requestTimeouts.writes,
    setClearingPreviewCache,
    setError,
    setNotice,
  ]);

  return {
    loadTypstPackages,
    onInstallTypstPackage,
    onDeleteTypstPackage,
    onClearTypstPreviewCache,
  };
}
