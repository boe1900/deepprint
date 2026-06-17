import { normalizeBaseUrl } from "./utils"
import type {
  AddPrinterResponse,
  ApiErrorBody,
  ClientSetupState,
  CupsConnectionTestResponse,
  CupsSettingsResponse,
  DeletePrinterResponse,
  DiscoveredPrintersResponse,
  PrinterDetail,
  PrinterSource,
  SaveClientSetupRequest,
  SaveDiagnosticBundleResponse,
  SavePreviewPdfResponse,
  ValidatedPrinterTarget,
} from "./types"

const DEFAULT_REQUEST_TIMEOUT_MS = 15_000
const CLIENT_SETUP_STORAGE_KEY = "deepprint.clientSetup"
export const REQUEST_TIMEOUT_MESSAGE =
  "请求超时，请检查 Agent URL、网络与服务状态"

export class RequestTimeoutError extends Error {
  timeoutMs: number
  path: string

  constructor(path: string, timeoutMs: number) {
    super(`请求超时（>${timeoutMs}ms）: ${path}`)
    this.name = "RequestTimeoutError"
    this.timeoutMs = timeoutMs
    this.path = path
  }
}

interface RequestJsonInit extends RequestInit {
  timeoutMs?: number
}

interface RequestBinaryInit extends RequestInit {
  timeoutMs?: number
}

interface RequestCommonInit extends RequestInit {
  timeoutMs?: number
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

function randomNonce(): string {
  const bytes = new Uint8Array(16)
  crypto.getRandomValues(bytes)
  return bytesToHex(bytes)
}

async function sha256Hex(value: string): Promise<string> {
  const data = new TextEncoder().encode(value)
  const digest = await crypto.subtle.digest("SHA-256", data)
  return bytesToHex(new Uint8Array(digest))
}

async function hmacSha256Hex(secret: string, payload: string): Promise<string> {
  const encoder = new TextEncoder()
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  )
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(payload))
  return bytesToHex(new Uint8Array(signature))
}

export interface BinaryResponse {
  payload: Uint8Array
  headers: Headers
  contentType: string
}

function createAbortController(signal?: AbortSignal | null): {
  controller: AbortController
  cleanup: () => void
} {
  const controller = new AbortController()

  if (!signal) {
    return {
      controller,
      cleanup: () => undefined,
    }
  }

  const relayAbort = () => controller.abort()
  if (signal.aborted) {
    controller.abort()
  } else {
    signal.addEventListener("abort", relayAbort, { once: true })
  }

  return {
    controller,
    cleanup: () => signal.removeEventListener("abort", relayAbort),
  }
}

function buildHeadersWithDefaults(requestInit: RequestInit): Headers {
  const headers = new Headers(requestInit.headers)
  const hasBody = requestInit.body !== undefined && requestInit.body !== null
  if (
    hasBody &&
    !(requestInit.body instanceof FormData) &&
    !headers.has("Content-Type")
  ) {
    headers.set("Content-Type", "application/json")
  }
  return headers
}

async function executeRequest(
  baseUrl: string,
  path: string,
  init?: RequestCommonInit
): Promise<Response> {
  const {
    signal: rawSignal,
    timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS,
    ...requestInit
  } = init ?? {}
  const headers = buildHeadersWithDefaults(requestInit)
  const { controller, cleanup } = createAbortController(rawSignal)

  let timeoutHandle: ReturnType<typeof setTimeout> | null = null
  let timedOut = false
  if (timeoutMs > 0) {
    timeoutHandle = setTimeout(() => {
      timedOut = true
      controller.abort()
    }, timeoutMs)
  }

  let response: Response
  try {
    response = await fetch(`${normalizeBaseUrl(baseUrl)}${path}`, {
      ...requestInit,
      headers,
      credentials: requestInit.credentials ?? "include",
      signal: controller.signal,
    })
  } catch (error) {
    if (timeoutHandle) {
      clearTimeout(timeoutHandle)
    }
    cleanup()

    if (timedOut) {
      throw new RequestTimeoutError(path, timeoutMs)
    }

    if (isAbortError(error)) {
      throw error
    }

    const message = error instanceof Error ? error.message : "网络请求失败"
    throw new Error(`网络请求失败: ${message}`)
  }

  if (timeoutHandle) {
    clearTimeout(timeoutHandle)
  }
  cleanup()

  if (timedOut) {
    throw new RequestTimeoutError(path, timeoutMs)
  }

  return response
}

async function parseJsonOrTextPayload(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") || ""
  try {
    if (contentType.includes("application/json")) {
      return await response.json()
    }
    return await response.text()
  } catch (error) {
    const message = error instanceof Error ? error.message : "响应解析失败"
    throw new Error(`响应解析失败: ${message}`)
  }
}

function buildHttpError(response: Response, payload: unknown): Error {
  const body = payload as ApiErrorBody | null
  const code = body?.code ? `${body.code}: ` : ""
  const message =
    typeof payload === "string"
      ? payload
      : body?.error || body?.message || `HTTP ${response.status}`
  return new Error(`${code}${message}`)
}

export async function requestJson<T>(
  baseUrl: string,
  path: string,
  init?: RequestJsonInit
): Promise<T> {
  const response = await executeRequest(baseUrl, path, init)
  const payload = await parseJsonOrTextPayload(response)
  if (!response.ok) {
    throw buildHttpError(response, payload)
  }

  return payload as T
}

export async function requestBinary(
  baseUrl: string,
  path: string,
  init?: RequestBinaryInit
): Promise<BinaryResponse> {
  const response = await executeRequest(baseUrl, path, init)
  if (!response.ok) {
    const payload = await parseJsonOrTextPayload(response)
    throw buildHttpError(response, payload)
  }

  let buffer: ArrayBuffer
  try {
    buffer = await response.arrayBuffer()
  } catch (error) {
    const message = error instanceof Error ? error.message : "响应解析失败"
    throw new Error(`响应解析失败: ${message}`)
  }

  return {
    payload: new Uint8Array(buffer),
    headers: response.headers,
    contentType: response.headers.get("content-type") || "",
  }
}

export function isAbortError(error: unknown): boolean {
  if (error instanceof DOMException && error.name === "AbortError") return true
  if (error instanceof Error) {
    return (
      error.name === "AbortError" ||
      error.message.toLowerCase().includes("aborted")
    )
  }
  return false
}

export function isTimeoutError(error: unknown): boolean {
  if (error instanceof RequestTimeoutError) return true
  if (error instanceof Error) {
    return error.name === "RequestTimeoutError"
  }
  return false
}

export function getRequestErrorMessage(
  error: unknown,
  fallbackMessage: string,
  timeoutMessage = REQUEST_TIMEOUT_MESSAGE
): string {
  if (isTimeoutError(error)) {
    return timeoutMessage
  }

  if (error instanceof Error) {
    const message = error.message.trim()
    return message || fallbackMessage
  }

  return fallbackMessage
}

function downloadBase64File(
  payloadBase64: string,
  fileName: string,
  contentType: string
) {
  const binary = atob(payloadBase64)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index)
  }

  const blob = new Blob([bytes], { type: contentType })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement("a")
  anchor.href = url
  anchor.download = fileName
  document.body.append(anchor)
  anchor.click()
  anchor.remove()
  URL.revokeObjectURL(url)
}

function downloadTextFile(payload: string, fileName: string) {
  const blob = new Blob([payload], { type: "text/plain;charset=utf-8" })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement("a")
  anchor.href = url
  anchor.download = fileName
  document.body.append(anchor)
  anchor.click()
  anchor.remove()
  URL.revokeObjectURL(url)
}

export async function getClientSetupState(): Promise<ClientSetupState | null> {
  try {
    const raw = window.localStorage.getItem(CLIENT_SETUP_STORAGE_KEY)
    return raw ? (JSON.parse(raw) as ClientSetupState) : null
  } catch {
    return null
  }
}

export async function saveClientSetupState(
  request: SaveClientSetupRequest
): Promise<ClientSetupState | null> {
  try {
    const state: ClientSetupState = {
      onboarding_completed: true,
      agent_base_url: request.agent_base_url,
      auth_enabled: request.auth_enabled,
      auth_use_keychain: false,
      auth_token_saved: Boolean(request.auth_token),
      auth_secret_saved: Boolean(request.auth_secret),
      updated_at: Math.floor(Date.now() / 1000),
    }
    window.localStorage.setItem(CLIENT_SETUP_STORAGE_KEY, JSON.stringify(state))
    return state
  } catch {
    return null
  }
}

export async function signClientWriteHeaders(
  method: string,
  path: string,
  bodyText: string,
  token: string | null,
  secret: string | null
): Promise<Record<string, string>> {
  if (!token || !secret) return {}

  const normalizedMethod = method.toUpperCase()
  const timestamp = Math.floor(Date.now() / 1000).toString()
  const nonce = randomNonce()
  const bodyHash = await sha256Hex(bodyText)
  const signingPayload = [
    normalizedMethod,
    path,
    timestamp,
    nonce,
    bodyHash,
  ].join("\n")
  const signature = await hmacSha256Hex(secret, signingPayload)

  return {
    "x-deepprint-token": token,
    "x-deepprint-ts": timestamp,
    "x-deepprint-nonce": nonce,
    "x-deepprint-signature": signature,
  }
}

export async function saveDiagnosticBundle(
  sourcePath: string,
  suggestedFileName?: string,
  cleanupSource = true,
  cleanupOnCancel = true
): Promise<SaveDiagnosticBundleResponse> {
  void cleanupSource
  void cleanupOnCancel
  downloadTextFile(sourcePath, suggestedFileName ?? "deepprint-diagnostics.txt")
  return {
    saved: true,
    destination_path: null,
    source_deleted: false,
  }
}

export async function savePreviewPdf(
  pdfBase64: string,
  suggestedFileName?: string
): Promise<SavePreviewPdfResponse> {
  downloadBase64File(
    pdfBase64,
    suggestedFileName ?? "deepprint-preview.pdf",
    "application/pdf"
  )
  return {
    saved: true,
    destination_path: null,
  }
}

export async function discoverCupsPrinters(
  baseUrl: string,
  timeoutMs?: number
): Promise<DiscoveredPrintersResponse> {
  return requestJson<DiscoveredPrintersResponse>(
    baseUrl,
    "/v1/printers/discover/cups",
    {
      timeoutMs,
    }
  )
}

export async function getCupsSettings(
  baseUrl: string,
  timeoutMs?: number
): Promise<CupsSettingsResponse> {
  return requestJson<CupsSettingsResponse>(baseUrl, "/v1/settings/cups", {
    timeoutMs,
  })
}

export async function updateCupsSettings(
  baseUrl: string,
  cupsBaseUrl: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<CupsSettingsResponse> {
  return requestJson<CupsSettingsResponse>(baseUrl, "/v1/settings/cups", {
    method: "POST",
    body: JSON.stringify({ cups_base_url: cupsBaseUrl }),
    headers: authHeaders,
    timeoutMs,
  })
}

export async function testCupsConnection(
  baseUrl: string,
  cupsBaseUrl: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<CupsConnectionTestResponse> {
  return requestJson<CupsConnectionTestResponse>(
    baseUrl,
    "/v1/settings/cups/test",
    {
      method: "POST",
      body: JSON.stringify({ cups_base_url: cupsBaseUrl }),
      headers: authHeaders,
      timeoutMs,
    }
  )
}

export async function validatePrinterUri(
  baseUrl: string,
  printerUri: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<ValidatedPrinterTarget> {
  return requestJson<ValidatedPrinterTarget>(baseUrl, "/v1/printers/validate", {
    method: "POST",
    body: JSON.stringify({ uri: printerUri }),
    headers: authHeaders,
    timeoutMs,
  })
}

export async function addPrinter(
  baseUrl: string,
  input: {
    source: PrinterSource
    printerUri: string
    displayName?: string
  },
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<AddPrinterResponse> {
  return requestJson<AddPrinterResponse>(baseUrl, "/v1/printers", {
    method: "POST",
    body: JSON.stringify({
      source: input.source,
      printer_uri: input.printerUri,
      display_name: input.displayName ?? "",
    }),
    headers: authHeaders,
    timeoutMs,
  })
}

export async function fetchPrinterDetail(
  baseUrl: string,
  printerId: string,
  timeoutMs?: number
): Promise<PrinterDetail> {
  return requestJson<PrinterDetail>(
    baseUrl,
    `/v1/printers/${encodeURIComponent(printerId)}`,
    {
      timeoutMs,
    }
  )
}

export async function refreshPrinter(
  baseUrl: string,
  printerId: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<PrinterDetail> {
  return requestJson<PrinterDetail>(
    baseUrl,
    `/v1/printers/${encodeURIComponent(printerId)}/refresh`,
    {
      method: "POST",
      body: JSON.stringify({}),
      headers: authHeaders,
      timeoutMs,
    }
  )
}

export async function setPrinterEnabled(
  baseUrl: string,
  printerId: string,
  enabled: boolean,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<PrinterDetail> {
  const action = enabled ? "enable" : "disable"
  return requestJson<PrinterDetail>(
    baseUrl,
    `/v1/printers/${encodeURIComponent(printerId)}/${action}`,
    {
      method: "POST",
      body: JSON.stringify({}),
      headers: authHeaders,
      timeoutMs,
    }
  )
}

export async function setDefaultPrinter(
  baseUrl: string,
  printerId: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<PrinterDetail> {
  return requestJson<PrinterDetail>(
    baseUrl,
    `/v1/printers/${encodeURIComponent(printerId)}/set-default`,
    {
      method: "POST",
      body: JSON.stringify({}),
      headers: authHeaders,
      timeoutMs,
    }
  )
}

export async function deletePrinter(
  baseUrl: string,
  printerId: string,
  timeoutMs: number,
  authHeaders: Record<string, string> = {}
): Promise<DeletePrinterResponse> {
  return requestJson<DeletePrinterResponse>(
    baseUrl,
    `/v1/printers/${encodeURIComponent(printerId)}`,
    {
      method: "DELETE",
      body: JSON.stringify({}),
      headers: authHeaders,
      timeoutMs,
    }
  )
}
