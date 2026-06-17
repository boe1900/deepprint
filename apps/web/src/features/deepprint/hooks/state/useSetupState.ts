import { useState } from "react";
import { DEFAULT_BASE_URL } from "../../constants";

export function useSetupState() {
  const [setupLoading, setSetupLoading] = useState(true);
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [setupSubmitting, setSetupSubmitting] = useState(false);
  const [setupError, setSetupError] = useState<string | null>(null);
  const [setupAgentBaseUrl, setSetupAgentBaseUrl] = useState(DEFAULT_BASE_URL);
  const [setupEnableAuth, setSetupEnableAuth] = useState(false);
  const [setupAuthTokenSaved, setSetupAuthTokenSaved] = useState(false);
  const [setupAuthSecretSaved, setSetupAuthSecretSaved] = useState(false);
  const [setupAuthToken, setSetupAuthToken] = useState("");
  const [setupAuthSecret, setSetupAuthSecret] = useState("");

  return {
    setupLoading,
    setSetupLoading,
    showOnboarding,
    setShowOnboarding,
    setupSubmitting,
    setSetupSubmitting,
    setupError,
    setSetupError,
    setupAgentBaseUrl,
    setSetupAgentBaseUrl,
    setupEnableAuth,
    setSetupEnableAuth,
    setupAuthTokenSaved,
    setSetupAuthTokenSaved,
    setupAuthSecretSaved,
    setSetupAuthSecretSaved,
    setupAuthToken,
    setSetupAuthToken,
    setupAuthSecret,
    setSetupAuthSecret,
  };
}
