export type NoticeKind = "ok" | "error"
export type JobTimelineSource = "manual" | "poll"
export type ErrorLevel = "critical" | "warn" | "info"
export type ThemeMode = "system" | "dark" | "light"

export interface RequestTimeoutSettings {
  health: number
  deepHealth: number
  printers: number
  jobStatus: number
  writes: number
  diagnosticsExport: number
  urlProbe: number
}

export interface NoticeState {
  kind: NoticeKind
  message: string
}

export interface BaseUrlProbeResult {
  ok: boolean
  message: string
  latency_ms: number
  normalized_base_url: string
  checked_at_ms: number
}

export interface HealthResponse {
  status: string
  version: string
  uptime_seconds: number
  mock_mode?: boolean
  cups_base_url?: string
  worker_concurrency?: number
  retry_max_attempts?: number
  retry_backoff_base_sec?: number
  retry_backoff_max_sec?: number
  queue_length: number
  rendering_jobs: number
  submitting_jobs?: number
  printing_jobs: number
  needs_attention_jobs?: number
  succeeded_total: number
  failed_total: number
  canceled_total: number
  terminal_total?: number
  success_rate: number
  failure_rate: number
  printer_backend?: string
  backend_name?: string
  render_engine: string
  auth_required_for_writes?: boolean
  auth_token_configured?: boolean
  auth_window_sec?: number
  auth_use_keychain?: boolean
  auth_keychain_service?: string
  direct_job_max_bytes?: number
  render_cache_entries?: number
  render_cache_disk_usage_bytes?: number
  avg_succeeded_duration_sec?: number
  log_dir?: string
  diagnostics_dir?: string
  typst_local_packages_root?: string
  typst_preview_cache_root?: string
  typst_fonts_root?: string
}

export interface HealthComponentProbe {
  ok: boolean
  latency_ms: number
  detail: string
}

export interface DeepHealthResponse {
  status: string
  overall_ok: boolean
  db: HealthComponentProbe
  backend?: HealthComponentProbe
  printer_backend?: HealthComponentProbe
  renderer_subprocess: HealthComponentProbe
}

export type PrinterSource = "manual" | "cups_import" | "mdns"

export interface PrinterInfo {
  id: string
  name: string
  uri: string
  source: PrinterSource
  is_default: boolean
  enabled: boolean
  state: string | null
  state_message: string | null
  last_validated_at: number | null
  last_seen_at: number | null
}

export interface PrinterCopiesCapability {
  default?: number | null
  min?: number | null
  max?: number | null
}

export interface PrinterCapabilities {
  document_formats: string[]
  media_supported: string[]
  media_default?: string | null
  media_types_supported: string[]
  sides_supported: string[]
  sides_default?: string | null
  copies?: PrinterCopiesCapability | null
  color_modes_supported: string[]
  color_supported?: boolean | null
  orientations_supported: string[]
  scalings_supported: string[]
  supports_page_ranges?: boolean | null
  job_creation_attributes_supported: string[]
}

export interface PrinterDetail extends PrinterInfo {
  normalized_uri: string
  capabilities: PrinterCapabilities
  attributes: unknown
  last_refreshed_at: number | null
  created_at: number
  updated_at: number
}

export interface PrintersResponse {
  printers: PrinterInfo[]
  note?: string
}

export interface AddPrinterResponse {
  printer: PrinterInfo
  created: boolean
}

export interface DeletePrinterResponse {
  deleted: boolean
}

export interface DiscoveredPrinter {
  display_name: string
  candidate_uri: string
  source: PrinterSource
  is_already_managed: boolean
  managed_printer_id: string | null
}

export interface DiscoveredPrintersResponse {
  printers: DiscoveredPrinter[]
  cups_base_url?: string | null
  reachable?: boolean | null
  message?: string | null
}

export interface CupsSettingsResponse {
  cups_base_url: string
  source: string
}

export interface CupsConnectionTestResponse {
  ok: boolean
  cups_base_url: string
  message: string
}

export interface ValidatedPrinterTarget {
  normalized_uri: string
  printer_uri: string
  discovered_name: string
  state: string | null
  state_message: string | null
  capabilities: PrinterCapabilities
  attributes: unknown
  already_managed: boolean
  managed_printer_id: string | null
}

export interface JobResponse {
  job_id: string
  request_id: string
  job_kind?: string
  printer_id?: string | null
  printer_name_snapshot?: string | null
  printer_uri?: string | null
  status: string
  attempt_count: number
  created_at?: number
  updated_at: number
  last_error_code: string | null
  last_error_message: string | null
  printer_backend?: string | null
  backend_name?: string | null
  backend_job_id?: string | null
  backend_job_ref_json?: string | null
  source_file_name?: string | null
  source_content_type?: string | null
  source_file_size_bytes?: number | null
}

export interface RecentJobsResponse {
  jobs: JobResponse[]
  limit: number
  printer_id?: string | null
}

export interface JobsListResponse {
  jobs: JobResponse[]
  page: number
  page_size: number
  total: number
  total_pages: number
  status_filter?: string[]
  printer_id?: string | null
  q?: string | null
  defaulted_to_needs_attention?: boolean
}

export interface TemplateRecord {
  id: string
  group_id: string
  name: string
  description: string
  output_name: string
  typst_code: string
  sample_data: string
  sort_order: number
  created_at: number
  updated_at: number
}

export interface TemplateGroup {
  id: string
  name: string
  sort_order: number
  created_at: number
  updated_at: number
  templates: TemplateRecord[]
}

export interface TemplateWorkspaceResponse {
  groups: TemplateGroup[]
}

export interface TemplateGroupResponse {
  group: TemplateGroup
}

export interface TemplateResponse {
  template: TemplateRecord
}

export interface DiagnosticExportResponse {
  bundle_id: string
  bundle_path: string
  size_bytes: number
  created_at: number
}

export interface SaveDiagnosticBundleResponse {
  saved: boolean
  destination_path: string | null
  source_deleted: boolean
}

export interface SavePreviewPdfResponse {
  saved: boolean
  destination_path: string | null
}

export interface CreateJobResponse {
  job_id: string
  status: string
  idempotent: boolean
}

export interface PreviewTypstResponse {
  output_kind: string
  page_count: number
  page_width_pt: number | null
  page_height_pt: number | null
}

export type TypstPackageOrigin = "local" | "preview_cache"

export interface TypstPackageInfo {
  origin: TypstPackageOrigin
  namespace: string
  name: string
  version: string
  import_snippet: string
}

export interface TypstPackagesResponse {
  packages: TypstPackageInfo[]
}

export interface TypstFontInfo {
  file_name: string
  size_bytes: number
  modified_at_ms: number | null
}

export interface TypstFontsResponse {
  fonts: TypstFontInfo[]
}

export interface InstallTypstPackageResponse {
  origin: TypstPackageOrigin
  namespace: string
  name: string
  version: string
  replaced: boolean
  import_snippet: string
}

export interface DeleteTypstPackageResponse {
  origin: TypstPackageOrigin
  namespace: string
  name: string
  version: string
  deleted: boolean
}

export interface ClearTypstPreviewCacheResponse {
  removed: boolean
}

export interface InstallTypstFontResponse {
  file_name: string
  size_bytes: number
  replaced: boolean
}

export interface DeleteTypstFontResponse {
  file_name: string
  deleted: boolean
}

export interface CancelJobResponse {
  job_id: string
  status: string
}

export interface DiagnosticHistoryItem extends DiagnosticExportResponse {
  base_url: string
  exported_at_ms: number
}

export type OpsProbeStatus = "idle" | "checking" | "ok" | "error"

export interface OpsProbeState {
  status: OpsProbeStatus
  message: string
  latency_ms: number | null
  checked_at_ms: number | null
}

export interface ApiErrorBody {
  code?: string
  error?: string
  message?: string
  details?: Record<string, unknown>
}

export interface ClientSetupState {
  onboarding_completed: boolean
  agent_base_url: string
  auth_enabled: boolean
  auth_use_keychain: boolean
  auth_token_saved: boolean
  auth_secret_saved: boolean
  updated_at: number
}

export interface SaveClientSetupRequest {
  agent_base_url: string
  auth_enabled: boolean
  auth_use_keychain: boolean
  auth_token: string | null
  auth_secret: string | null
}

export interface SignClientWriteHeadersResponse {
  token: string
  timestamp: string
  nonce: string
  signature: string
}

export interface JobTimelineEntry {
  status: string
  attempt_count: number
  updated_at: number
  last_error_code: string | null
  last_error_message: string | null
  source: JobTimelineSource
}

export interface JobErrorCategory {
  label: string
  level: ErrorLevel
  hint: string
}
