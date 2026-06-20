import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentLocale, translate } from "@/i18n";
import { getRequestErrorMessage, isAbortError, requestJson, signClientWriteHeaders } from "../api";
import { createTypstFontsQueryOptions, deepprintQueryKeys } from "../queries";
import { useLatestAsyncTask } from "./useLatestAsyncTask";
import { readFileAsBase64 } from "../utils";
import type {
  DeleteTypstFontResponse,
  InstallTypstFontResponse,
  NoticeState,
  RequestTimeoutSettings,
  TypstFontInfo,
} from "../types";

interface UseTypstFontsActionsArgs {
  baseUrl: string;
  requestTimeouts: RequestTimeoutSettings;
  authRequiredForWrites: boolean;
  writeAuthToken: string;
  writeAuthSecret: string;
  setNotice: (notice: NoticeState) => void;
  setFonts: (value: TypstFontInfo[]) => void;
  setLoading: (value: boolean) => void;
  setError: (value: string | null) => void;
  setInstalling: (value: boolean) => void;
  setDeletingName: (value: string | null) => void;
}

interface UseTypstFontsActionsResult {
  loadTypstFonts: (silent?: boolean) => Promise<void>;
  onInstallTypstFont: (file: File) => Promise<void>;
  onDeleteTypstFont: (font: TypstFontInfo) => Promise<void>;
}

export function useTypstFontsActions({
  baseUrl,
  requestTimeouts,
  authRequiredForWrites,
  writeAuthToken,
  writeAuthSecret,
  setNotice,
  setFonts,
  setLoading,
  setError,
  setInstalling,
  setDeletingName,
}: UseTypstFontsActionsArgs): UseTypstFontsActionsResult {
  const queryClient = useQueryClient();
  const installTask = useLatestAsyncTask();
  const deleteTask = useLatestAsyncTask();
  const fontsQuery = useQuery({
    ...createTypstFontsQueryOptions(baseUrl, requestTimeouts.writes),
  });

  useEffect(() => {
    if (!fontsQuery.data) return;
    setFonts(fontsQuery.data.fonts);
    setError(null);
  }, [fontsQuery.data, setError, setFonts]);

  useEffect(() => {
    setLoading(fontsQuery.isFetching);
  }, [fontsQuery.isFetching, setLoading]);

  const prepareWriteHeaders = useCallback(
    async (method: string, path: string, bodyText: string): Promise<Record<string, string>> => {
      const token = writeAuthToken.trim();
      const secret = writeAuthSecret.trim();
      const hasAnyAuthInput = token.length > 0 || secret.length > 0;

      if (!authRequiredForWrites && !hasAnyAuthInput) {
        return {};
      }

      if (hasAnyAuthInput && (!token || !secret)) {
        throw new Error(tr("typstAssets.authTokenSecretRequired"));
      }

      try {
        return await signClientWriteHeaders(method, path, bodyText, token || null, secret || null);
      } catch (error) {
        const message = error instanceof Error ? error.message : tr("typstAssets.signFailed");
        if (authRequiredForWrites) {
          throw new Error(
            tr("typstAssets.authRequiredSignFailed", { message }),
          );
        }
        throw new Error(tr("typstAssets.signFailedWithMessage", { message }));
      }
    },
    [authRequiredForWrites, writeAuthSecret, writeAuthToken],
  );

  const loadTypstFonts = useCallback(
    async (silent = false) => {
      if (!silent) {
        setError(null);
      }

      try {
        await queryClient.fetchQuery({
          ...createTypstFontsQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
      } catch (error) {
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("typstAssets.loadFontsFailed"));
        setError(message);
        if (!silent) {
          setNotice({ kind: "error", message: tr("typstAssets.loadFontsFailedWithMessage", { message }) });
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

  const onInstallTypstFont = useCallback(
    async (file: File) => {
      const ticket = installTask.start();
      setInstalling(true);
      setError(null);

      try {
        const fileContentBase64 = await readFileAsBase64(file);
        const path = "/v1/typst/fonts/install";
        const bodyText = JSON.stringify({
          file_base64: fileContentBase64,
          file_name: file.name,
          replace_existing: true,
        });
        const headers = await prepareWriteHeaders("POST", path, bodyText);
        await requestJson<InstallTypstFontResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: requestTimeouts.writes,
        });

        if (!ticket.isCurrent()) return;
        await queryClient.invalidateQueries({
          queryKey: deepprintQueryKeys.typstFonts(baseUrl),
        });
        await queryClient.fetchQuery({
          ...createTypstFontsQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
        setNotice({ kind: "ok", message: tr("typstAssets.installFontNotice", { name: file.name }) });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("typstAssets.installFontFailed"));
        setError(message);
        setNotice({ kind: "error", message: tr("typstAssets.installFontFailedWithMessage", { message }) });
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

  const onDeleteTypstFont = useCallback(
    async (font: TypstFontInfo) => {
      const ticket = deleteTask.start();
      setDeletingName(font.file_name);

      try {
        const path = "/v1/typst/fonts/delete";
        const bodyText = JSON.stringify({
          file_name: font.file_name,
        });
        const headers = await prepareWriteHeaders("POST", path, bodyText);
        await requestJson<DeleteTypstFontResponse>(baseUrl, path, {
          method: "POST",
          body: bodyText,
          headers,
          signal: ticket.signal,
          timeoutMs: requestTimeouts.writes,
        });

        if (!ticket.isCurrent()) return;
        await queryClient.invalidateQueries({
          queryKey: deepprintQueryKeys.typstFonts(baseUrl),
        });
        await queryClient.fetchQuery({
          ...createTypstFontsQueryOptions(baseUrl, requestTimeouts.writes),
          staleTime: 0,
        });
        setNotice({ kind: "ok", message: tr("typstAssets.deleteFontNotice", { name: font.file_name }) });
      } catch (error) {
        if (!ticket.isCurrent()) return;
        if (isAbortError(error)) return;

        const message = getRequestErrorMessage(error, tr("typstAssets.deleteFontFailed"));
        setError(message);
        setNotice({ kind: "error", message: tr("typstAssets.deleteFontFailedWithMessage", { message }) });
      } finally {
        ticket.finish();
        if (ticket.isCurrent()) {
          setDeletingName(null);
        }
      }
    },
    [
      baseUrl,
      deleteTask,
      prepareWriteHeaders,
      queryClient,
      requestTimeouts.writes,
      setDeletingName,
      setError,
      setNotice,
    ],
  );

  return {
    loadTypstFonts,
    onInstallTypstFont,
    onDeleteTypstFont,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
