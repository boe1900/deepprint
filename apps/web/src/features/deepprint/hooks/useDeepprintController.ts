import { useDeepprintPersistenceEffects } from "./useDeepprintPersistenceEffects";
import { useDeepprintActions } from "./useDeepprintActions";
import { useAgentState } from "./state/useAgentState";
import { useDiagnosticsState } from "./state/useDiagnosticsState";
import { useJobsState } from "./state/useJobsState";
import { useSetupState } from "./state/useSetupState";
import { useTypstFontsState } from "./state/useTypstFontsState";
import { useTypstPackagesState } from "./state/useTypstPackagesState";
import { useUiState } from "./state/useUiState";
import { useWritesState } from "./state/useWritesState";

export function useDeepprintController() {
  const ui = useUiState();
  const setup = useSetupState();
  const agent = useAgentState();
  const jobs = useJobsState();
  const writes = useWritesState();
  const diagnostics = useDiagnosticsState();
  const typstPackages = useTypstPackagesState();
  const typstFonts = useTypstFontsState();

  useDeepprintPersistenceEffects({
    baseUrl: ui.baseUrl,
    diagHistory: diagnostics.diagHistory,
    previewPdfUrl: writes.previewPdfUrl,
    themeMode: ui.themeMode,
    requestTimeouts: ui.requestTimeouts,
  });

  const actions = useDeepprintActions({
    ui,
    setup,
    agent,
    jobs,
    writes,
    diagnostics,
    typstPackages,
    typstFonts,
  });

  const writeAuthReady =
    writes.writeAuthToken.trim().length > 0 && writes.writeAuthSecret.trim().length > 0;
  const hostSavedAuthReady = setup.setupAuthTokenSaved && setup.setupAuthSecretSaved;

  return {
    ui,
    setup,
    agent,
    jobs,
    writes,
    diagnostics,
    typstPackages,
    typstFonts,
    actions,
    authRequiredForWrites: actions.authRequiredForWrites,
    writeAuthReady,
    hostSavedAuthReady,
  };
}
