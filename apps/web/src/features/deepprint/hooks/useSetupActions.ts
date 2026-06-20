import { useCallback, useEffect } from "react";
import { getCurrentLocale, translate } from "@/i18n";
import { getClientSetupState, saveClientSetupState } from "../api";
import { DEFAULT_BASE_URL } from "../constants";
import { normalizeBaseUrl } from "../utils";
import type { NoticeState } from "../types";

interface UseSetupActionsArgs {
  setupAgentBaseUrl: string;
  setupEnableAuth: boolean;
  setupAuthToken: string;
  setupAuthSecret: string;
  refreshAll: (silent: boolean) => Promise<void>;
  setBaseUrl: (value: string) => void;
  setSetupAgentBaseUrl: (value: string) => void;
  setSetupEnableAuth: (value: boolean) => void;
  setSetupAuthTokenSaved: (value: boolean) => void;
  setSetupAuthSecretSaved: (value: boolean) => void;
  setSetupAuthToken: (value: string) => void;
  setSetupAuthSecret: (value: string) => void;
  setWriteAuthToken: (value: string) => void;
  setWriteAuthSecret: (value: string) => void;
  setShowOnboarding: (value: boolean) => void;
  setSetupLoading: (value: boolean) => void;
  setSetupSubmitting: (value: boolean) => void;
  setSetupError: (value: string | null) => void;
  setNotice: (notice: NoticeState) => void;
}

interface UseSetupActionsResult {
  onSubmitOnboarding: (useDefault: boolean) => Promise<void>;
}

export function useSetupActions({
  setupAgentBaseUrl,
  setupEnableAuth,
  setupAuthToken,
  setupAuthSecret,
  refreshAll,
  setBaseUrl,
  setSetupAgentBaseUrl,
  setSetupEnableAuth,
  setSetupAuthTokenSaved,
  setSetupAuthSecretSaved,
  setSetupAuthToken,
  setSetupAuthSecret,
  setWriteAuthToken,
  setWriteAuthSecret,
  setShowOnboarding,
  setSetupLoading,
  setSetupSubmitting,
  setSetupError,
  setNotice,
}: UseSetupActionsArgs): UseSetupActionsResult {
  useEffect(() => {
    void (async () => {
      const setupState = await getClientSetupState();
      if (!setupState) {
        setSetupLoading(false);
        setShowOnboarding(false);
        return;
      }

      const nextBaseUrl = normalizeBaseUrl(setupState.agent_base_url || DEFAULT_BASE_URL);
      setBaseUrl(nextBaseUrl);
      setSetupAgentBaseUrl(nextBaseUrl);
      setSetupEnableAuth(setupState.auth_enabled);
      setSetupAuthTokenSaved(setupState.auth_token_saved);
      setSetupAuthSecretSaved(setupState.auth_secret_saved);
      setSetupAuthToken("");
      setSetupAuthSecret("");
      setWriteAuthToken("");
      setWriteAuthSecret("");
      setShowOnboarding(!setupState.onboarding_completed);
      setSetupLoading(false);
    })();
  }, [
    setBaseUrl,
    setSetupAgentBaseUrl,
    setSetupAuthSecret,
    setSetupAuthSecretSaved,
    setSetupAuthToken,
    setSetupAuthTokenSaved,
    setSetupEnableAuth,
    setSetupLoading,
    setShowOnboarding,
    setWriteAuthSecret,
    setWriteAuthToken,
  ]);

  const onSubmitOnboarding = useCallback(
    async (useDefault: boolean) => {
      setSetupSubmitting(true);
      setSetupError(null);
      try {
        const normalizedBaseUrl = normalizeBaseUrl(
          useDefault ? DEFAULT_BASE_URL : setupAgentBaseUrl,
        );
        const enableAuth = useDefault ? false : setupEnableAuth;
        const token = (useDefault ? "" : setupAuthToken).trim();
        const secret = (useDefault ? "" : setupAuthSecret).trim();

        if (enableAuth && (!token || !secret)) {
          throw new Error(tr("setup.authRequiredTokenSecret"));
        }

        const saved = await saveClientSetupState({
          agent_base_url: normalizedBaseUrl,
          auth_enabled: enableAuth,
          auth_use_keychain: true,
          auth_token: token || null,
          auth_secret: secret || null,
        });

        if (!saved) {
          throw new Error(tr("setup.saveUnsupported"));
        }

        const keepTokenInUi = !saved.auth_token_saved && token.length > 0;
        const keepSecretInUi = !saved.auth_secret_saved && secret.length > 0;

        setBaseUrl(normalizedBaseUrl);
        setWriteAuthToken(keepTokenInUi ? token : "");
        setWriteAuthSecret(keepSecretInUi ? secret : "");
        setSetupAgentBaseUrl(normalizedBaseUrl);
        setSetupEnableAuth(saved.auth_enabled);
        setSetupAuthTokenSaved(saved.auth_token_saved || Boolean(token));
        setSetupAuthSecretSaved(saved.auth_secret_saved || Boolean(secret));
        setSetupAuthToken("");
        setSetupAuthSecret("");
        setShowOnboarding(false);
        setNotice({ kind: "ok", message: tr("setup.succeeded") });
        void refreshAll(true);
      } catch (error) {
        const message = error instanceof Error ? error.message : tr("setup.saveFailed");
        setSetupError(message);
      } finally {
        setSetupSubmitting(false);
        setSetupLoading(false);
      }
    },
    [
      refreshAll,
      setBaseUrl,
      setNotice,
      setSetupAgentBaseUrl,
      setSetupAuthSecret,
      setSetupAuthSecretSaved,
      setSetupAuthToken,
      setSetupAuthTokenSaved,
      setSetupEnableAuth,
      setSetupError,
      setSetupLoading,
      setSetupSubmitting,
      setShowOnboarding,
      setWriteAuthSecret,
      setWriteAuthToken,
      setupAgentBaseUrl,
      setupAuthSecret,
      setupAuthToken,
      setupEnableAuth,
    ],
  );

  return {
    onSubmitOnboarding,
  };
}

function tr(key: string, params?: Record<string, string | number | null | undefined>) {
  return translate(getCurrentLocale(), key, params);
}
