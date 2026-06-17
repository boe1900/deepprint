const DEFAULT_REQUEST_TIMEOUT_MS = 8_000

export type HealthResponse = {
  status: string
  version?: string
  uptime_seconds?: number
  queue_length?: number
  render_engine?: string
}

export type ProbeRow = {
  id: string
  label: string
  status: string
  detail: string
}

function normalizeBaseUrl(value: string) {
  const trimmed = value.trim()
  if (!trimmed || trimmed === "/") return ""
  return trimmed.endsWith("/") ? trimmed.slice(0, -1) : trimmed
}

async function requestJson<T>(baseUrl: string, path: string): Promise<T> {
  const controller = new AbortController()
  const timeout = window.setTimeout(() => controller.abort(), DEFAULT_REQUEST_TIMEOUT_MS)

  try {
    const response = await fetch(`${normalizeBaseUrl(baseUrl)}${path}`, {
      signal: controller.signal,
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`)
    }

    return (await response.json()) as T
  } finally {
    window.clearTimeout(timeout)
  }
}

export function getDefaultApiBaseUrl() {
  return import.meta.env.VITE_DEEPPRINT_API_BASE_URL || ""
}

export function fetchHealth(baseUrl: string) {
  return requestJson<HealthResponse>(baseUrl, "/v1/health")
}

export function buildProbeRows(
  health?: HealthResponse
): ProbeRow[] {
  return [
    {
      id: "server",
      label: "Server",
      status: health?.status ?? "unknown",
      detail: health
        ? `version ${health.version ?? "unknown"} / queue ${health.queue_length ?? 0}`
        : "Waiting for /v1/health",
    },
  ]
}
