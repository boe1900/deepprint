import { useAgentReadActions } from "./useAgentReadActions";
import { useConsoleOpsActions } from "./useConsoleOpsActions";
import { useJobsActions } from "./useJobsActions";
import { useSetupActions } from "./useSetupActions";
import { useTypstFontsActions } from "./useTypstFontsActions";
import { useTypstPackagesActions } from "./useTypstPackagesActions";
import { useWritesActions } from "./useWritesActions";
import type { useAgentState } from "./state/useAgentState";
import type { useDiagnosticsState } from "./state/useDiagnosticsState";
import type { useJobsState } from "./state/useJobsState";
import type { useSetupState } from "./state/useSetupState";
import type { useTypstFontsState } from "./state/useTypstFontsState";
import type { useTypstPackagesState } from "./state/useTypstPackagesState";
import type { useUiState } from "./state/useUiState";
import type { useWritesState } from "./state/useWritesState";

type UiState = ReturnType<typeof useUiState>;
type SetupState = ReturnType<typeof useSetupState>;
type AgentState = ReturnType<typeof useAgentState>;
type JobsState = ReturnType<typeof useJobsState>;
type WritesState = ReturnType<typeof useWritesState>;
type DiagnosticsState = ReturnType<typeof useDiagnosticsState>;
type TypstPackagesState = ReturnType<typeof useTypstPackagesState>;
type TypstFontsState = ReturnType<typeof useTypstFontsState>;

interface UseDeepprintActionsArgs {
  ui: UiState;
  setup: SetupState;
  agent: AgentState;
  jobs: JobsState;
  writes: WritesState;
  diagnostics: DiagnosticsState;
  typstPackages: TypstPackagesState;
  typstFonts: TypstFontsState;
}

export function useDeepprintActions({
  ui,
  setup,
  agent,
  jobs,
  writes,
  diagnostics,
  typstPackages,
  typstFonts,
}: UseDeepprintActionsArgs) {
  const { loadPrinters, refreshAll } = useAgentReadActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    setupLoading: setup.setupLoading,
    showOnboarding: setup.showOnboarding,
    autoRefresh: ui.autoRefresh,
    setHealth: agent.setHealth,
    setDeepHealth: agent.setDeepHealth,
    setPrinters: agent.setPrinters,
    setPrintersNote: agent.setPrintersNote,
    setLoadingHealth: agent.setLoadingHealth,
    setLoadingPrinters: agent.setLoadingPrinters,
    setRefreshingAll: agent.setRefreshingAll,
    setLastRefreshAt: ui.setLastRefreshAt,
    setNotice: ui.setNotice,
  });

  const { onSubmitOnboarding } = useSetupActions({
    setupAgentBaseUrl: setup.setupAgentBaseUrl,
    setupEnableAuth: setup.setupEnableAuth,
    setupAuthToken: setup.setupAuthToken,
    setupAuthSecret: setup.setupAuthSecret,
    refreshAll,
    setBaseUrl: ui.setBaseUrl,
    setSetupAgentBaseUrl: setup.setSetupAgentBaseUrl,
    setSetupEnableAuth: setup.setSetupEnableAuth,
    setSetupAuthTokenSaved: setup.setSetupAuthTokenSaved,
    setSetupAuthSecretSaved: setup.setSetupAuthSecretSaved,
    setSetupAuthToken: setup.setSetupAuthToken,
    setSetupAuthSecret: setup.setSetupAuthSecret,
    setWriteAuthToken: writes.setWriteAuthToken,
    setWriteAuthSecret: writes.setWriteAuthSecret,
    setShowOnboarding: setup.setShowOnboarding,
    setSetupLoading: setup.setSetupLoading,
    setSetupSubmitting: setup.setSetupSubmitting,
    setSetupError: setup.setSetupError,
    setNotice: ui.setNotice,
  });

  const { fetchJobById, onLookupJob, onRefreshCurrentJob } = useJobsActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    jobIdInput: jobs.jobIdInput,
    jobAutoPoll: jobs.jobAutoPoll,
    jobPollIntervalSec: jobs.jobPollIntervalSec,
    currentJobId: jobs.jobResult?.job_id,
    currentJobStatus: jobs.jobResult?.status,
    setJobLoading: jobs.setJobLoading,
    setJobError: jobs.setJobError,
    setJobPollError: jobs.setJobPollError,
    setJobResult: jobs.setJobResult,
    setJobTimeline: jobs.setJobTimeline,
    setNotice: ui.setNotice,
  });

  const authRequiredForWrites = Boolean(agent.health?.auth_required_for_writes);

  const {
    onCreateJob,
    onPreviewTypst,
    onDismissPreview,
    onCreateDirectJob,
    onCancelJob,
    onExportDiagnostics,
    onResetWriteForms,
  } = useWritesActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    authRequiredForWrites,
    writeAuthToken: writes.writeAuthToken,
    writeAuthSecret: writes.writeAuthSecret,
    createRequestId: writes.createRequestId,
    createTemplateContent: writes.createTemplateContent,
    createDataJson: writes.createDataJson,
    createPrinterId: writes.createPrinterId,
    createCopies: writes.createCopies,
    createPaperSize: writes.createPaperSize,
    createDuplex: writes.createDuplex,
    directPrinterId: writes.directPrinterId,
    directSelectedFile: writes.directSelectedFile,
    directJobMaxBytes: agent.health?.direct_job_max_bytes,
    cancelTargetJobId: writes.cancelTargetJobId,
    currentJobId: jobs.jobResult?.job_id,
    latestCreatedJobId: writes.createResult?.job_id,
    jobIdInput: jobs.jobIdInput,
    fetchJobById,
    setNotice: ui.setNotice,
    setJobIdInput: jobs.setJobIdInput,
    setJobTimeline: jobs.setJobTimeline,
    setCancelTargetJobId: writes.setCancelTargetJobId,
    setCreateRequestId: writes.setCreateRequestId,
    setCreateTemplateContent: writes.setCreateTemplateContent,
    setCreateDataJson: writes.setCreateDataJson,
    setCreatePrinterId: writes.setCreatePrinterId,
    setCreateCopies: writes.setCreateCopies,
    setCreatePaperSize: writes.setCreatePaperSize,
    setCreateDuplex: writes.setCreateDuplex,
    setCreateLoading: writes.setCreateLoading,
    setCreateError: writes.setCreateError,
    setCreateResult: writes.setCreateResult,
    setPreviewLoading: writes.setPreviewLoading,
    setPreviewError: writes.setPreviewError,
    setPreviewResult: writes.setPreviewResult,
    setPreviewPdfUrl: writes.setPreviewPdfUrl,
    setPreviewModalOpen: writes.setPreviewModalOpen,
    setDirectPrinterId: writes.setDirectPrinterId,
    setDirectSelectedFile: writes.setDirectSelectedFile,
    setDirectFileInputKey: writes.setDirectFileInputKey,
    setDirectLoading: writes.setDirectLoading,
    setDirectError: writes.setDirectError,
    setDirectResult: writes.setDirectResult,
    setCancelLoading: writes.setCancelLoading,
    setCancelError: writes.setCancelError,
    setCancelResult: writes.setCancelResult,
    setDiagLoading: diagnostics.setDiagLoading,
    setDiagResult: diagnostics.setDiagResult,
    setDiagHistory: diagnostics.setDiagHistory,
  });

  const {
    onProbeBaseUrl,
    onRunOpsProbe,
    onCopyDiagnosticPath,
    onClearDiagnosticsHistory,
    onClearAllDiagnosticsHistory,
    onDeleteDiagnosticHistoryItem,
    onResetBaseUrl,
    onResetViewSettings,
    onResetRequestTimeouts,
  } = useConsoleOpsActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    setOpsProbe: diagnostics.setOpsProbe,
    setNotice: ui.setNotice,
    setBaseUrlProbe: ui.setBaseUrlProbe,
    setDiagHistory: diagnostics.setDiagHistory,
    setBaseUrl: ui.setBaseUrl,
    setAutoRefresh: ui.setAutoRefresh,
    setJobAutoPoll: jobs.setJobAutoPoll,
    setJobPollIntervalSec: jobs.setJobPollIntervalSec,
    setRequestTimeouts: ui.setRequestTimeouts,
  });

  const {
    loadTypstPackages,
    onInstallTypstPackage,
    onDeleteTypstPackage,
    onClearTypstPreviewCache,
  } = useTypstPackagesActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    authRequiredForWrites,
    writeAuthToken: writes.writeAuthToken,
    writeAuthSecret: writes.writeAuthSecret,
    setNotice: ui.setNotice,
    setPackages: typstPackages.setPackages,
    setLoading: typstPackages.setLoading,
    setError: typstPackages.setError,
    setInstalling: typstPackages.setInstalling,
    setDeletingKey: typstPackages.setDeletingKey,
    setClearingPreviewCache: typstPackages.setClearingPreviewCache,
  });

  const { loadTypstFonts, onInstallTypstFont, onDeleteTypstFont } = useTypstFontsActions({
    baseUrl: ui.baseUrl,
    requestTimeouts: ui.requestTimeouts,
    authRequiredForWrites,
    writeAuthToken: writes.writeAuthToken,
    writeAuthSecret: writes.writeAuthSecret,
    setNotice: ui.setNotice,
    setFonts: typstFonts.setFonts,
    setLoading: typstFonts.setLoading,
    setError: typstFonts.setError,
    setInstalling: typstFonts.setInstalling,
    setDeletingName: typstFonts.setDeletingName,
  });

  return {
    authRequiredForWrites,
    loadPrinters,
    refreshAll,
    onSubmitOnboarding,
    onLookupJob,
    onRefreshCurrentJob,
    onCreateJob,
    onPreviewTypst,
    onDismissPreview,
    onCreateDirectJob,
    onCancelJob,
    onExportDiagnostics,
    onResetWriteForms,
    onProbeBaseUrl,
    onRunOpsProbe,
    onCopyDiagnosticPath,
    onClearDiagnosticsHistory,
    onClearAllDiagnosticsHistory,
    onDeleteDiagnosticHistoryItem,
    onResetBaseUrl,
    onResetViewSettings,
    onResetRequestTimeouts,
    loadTypstPackages,
    onInstallTypstPackage,
    onDeleteTypstPackage,
    onClearTypstPreviewCache,
    loadTypstFonts,
    onInstallTypstFont,
    onDeleteTypstFont,
  };
}
