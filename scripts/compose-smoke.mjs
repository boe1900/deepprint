#!/usr/bin/env node

const serverBaseUrl =
  process.env.DEEPPRINT_SMOKE_SERVER_BASE_URL ||
  process.env.DEEPPRINT_SERVER_BASE_URL ||
  "http://127.0.0.1:17801"
const webBaseUrl =
  process.env.DEEPPRINT_SMOKE_WEB_BASE_URL ||
  process.env.DEEPPRINT_WEB_BASE_URL ||
  "http://127.0.0.1:8080"
const adminUsername = (process.env.DEEPPRINT_SMOKE_ADMIN_USERNAME || "admin").trim()
const bootstrapPassword =
  process.env.DEEPPRINT_SMOKE_ADMIN_PASSWORD || "example-bootstrap-password-123!"
const rotatedPassword =
  process.env.DEEPPRINT_SMOKE_ADMIN_NEW_PASSWORD || "example-rotated-password-123!"
const preferredPrinterName = (
  process.env.DEEPPRINT_SMOKE_PRINTER_NAME ||
  process.env.DEEPPRINT_CUPS_PDF_PRINTER_NAME ||
  "CUPS-PDF"
).trim()
const pollIntervalMs = parsePositiveInt(
  process.env.DEEPPRINT_SMOKE_POLL_INTERVAL_MS,
  800,
)
const timeoutMs = parsePositiveInt(process.env.DEEPPRINT_SMOKE_TIMEOUT_MS, 120000)
const requestTimeoutMs = parsePositiveInt(
  process.env.DEEPPRINT_SMOKE_REQUEST_TIMEOUT_MS,
  15000,
)

const runId = `${Date.now()}-${Math.floor(Math.random() * 10000)}`
const templateGroupName = `Compose Smoke ${runId}`
const templateName = `Smoke Template ${runId}`
const apiKeyName = `compose-smoke-${runId}`
const openRequestId = `compose-open-${runId}`

class HttpError extends Error {
  constructor(message, status, payload) {
    super(message)
    this.name = "HttpError"
    this.status = status
    this.payload = payload
  }
}

class CookieJar {
  constructor() {
    this.cookies = new Map()
  }

  absorb(headers) {
    const setCookies =
      typeof headers.getSetCookie === "function"
        ? headers.getSetCookie()
        : headers.get("set-cookie")
          ? [headers.get("set-cookie")]
          : []

    for (const cookie of setCookies) {
      if (!cookie) continue
      const [pair] = cookie.split(";")
      const separator = pair.indexOf("=")
      if (separator <= 0) continue
      const name = pair.slice(0, separator).trim()
      const value = pair.slice(separator + 1).trim()
      if (!name) continue
      this.cookies.set(name, value)
    }
  }

  toHeaderValue() {
    return Array.from(this.cookies.entries())
      .map(([name, value]) => `${name}=${value}`)
      .join("; ")
  }
}

function parsePositiveInt(raw, fallback) {
  if (!raw) return fallback
  const parsed = Number.parseInt(raw, 10)
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback
  return parsed
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function logStep(message) {
  console.log(`[compose-smoke] ${message}`)
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

async function request(path, init = {}) {
  const {
    baseUrl = webBaseUrl,
    method = "GET",
    json,
    headers = {},
    cookieJar,
    timeout = requestTimeoutMs,
    expectBinary = false,
    throwOnHttpError = true,
  } = init

  const controller = new AbortController()
  const timeoutHandle = setTimeout(() => controller.abort(), timeout)
  const requestHeaders = new Headers(headers)
  if (json !== undefined) {
    requestHeaders.set("content-type", "application/json")
  }

  if (cookieJar) {
    const cookieHeader = cookieJar.toHeaderValue()
    if (cookieHeader) {
      requestHeaders.set("cookie", cookieHeader)
    }
  }

  let response
  try {
    response = await fetch(`${baseUrl}${path}`, {
      method,
      headers: requestHeaders,
      body: json === undefined ? undefined : JSON.stringify(json),
      signal: controller.signal,
    })
  } catch (error) {
    if (error instanceof Error && error.name === "AbortError") {
      throw new Error(`request timed out after ${timeout}ms: ${method} ${baseUrl}${path}`)
    }
    throw error
  } finally {
    clearTimeout(timeoutHandle)
  }

  if (cookieJar) {
    cookieJar.absorb(response.headers)
  }

  let payload
  if (expectBinary) {
    const buffer = Buffer.from(await response.arrayBuffer())
    payload = buffer
  } else {
    const contentType = response.headers.get("content-type") || ""
    if (contentType.includes("application/json")) {
      payload = await response.json()
    } else {
      payload = await response.text()
    }
  }

  if (!response.ok && throwOnHttpError) {
    const detail =
      typeof payload === "string"
        ? payload
        : payload?.message || payload?.error || JSON.stringify(payload)
    throw new HttpError(
      `HTTP ${response.status} for ${method} ${path}: ${detail}`,
      response.status,
      payload,
    )
  }

  return {
    status: response.status,
    headers: response.headers,
    payload,
  }
}

async function waitForJson(name, path, init, predicate) {
  const startedAt = Date.now()
  let lastError = null

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const result = await request(path, init)
      if (predicate(result.payload)) {
        return result.payload
      }
      lastError = new Error(`${name} returned unexpected payload`)
    } catch (error) {
      lastError = error
    }
    await sleep(pollIntervalMs)
  }

  const detail = lastError instanceof Error ? lastError.message : String(lastError)
  throw new Error(`timed out waiting for ${name}: ${detail}`)
}

async function loginWithFallback(cookieJar) {
  for (const password of [bootstrapPassword, rotatedPassword]) {
    const result = await request("/v1/auth/login", {
      method: "POST",
      json: { username: adminUsername, password },
      cookieJar,
      throwOnHttpError: false,
    })

    if (result.status === 200) {
      return {
        password,
        response: result.payload,
      }
    }

    if (result.status !== 401) {
      const detail =
        typeof result.payload === "string"
          ? result.payload
          : result.payload?.message || JSON.stringify(result.payload)
      throw new Error(`login failed with unexpected status ${result.status}: ${detail}`)
    }
  }

  throw new Error("login failed with both bootstrap and rotated admin passwords")
}

async function pollOpenJob(token, jobId) {
  const startedAt = Date.now()
  while (Date.now() - startedAt < timeoutMs) {
    const result = await request(`/v1/open/jobs/${encodeURIComponent(jobId)}`, {
      headers: { authorization: `Bearer ${token}` },
    })
    const status = result.payload?.status
    if (status === "succeeded") {
      return result.payload
    }
    if (status === "failed" || status === "canceled") {
      throw new Error(`open job finished with status=${status}`)
    }
    await sleep(pollIntervalMs)
  }

  throw new Error(`timed out waiting for open job ${jobId}`)
}

function selectDiscoveredPrinter(printers) {
  if (!Array.isArray(printers) || printers.length === 0) return null

  const normalizedPreferredName = preferredPrinterName.toLowerCase()
  return (
    printers.find((printer) =>
      String(printer.display_name || "")
        .trim()
        .toLowerCase()
        .includes(normalizedPreferredName),
    ) ||
    printers.find((printer) =>
      String(printer.candidate_uri || "")
        .trim()
        .toLowerCase()
        .includes(normalizedPreferredName),
    ) ||
    printers[0]
  )
}

async function registerManagedPrinter(cookieJar, printerInput) {
  const printerResult = await request("/v1/printers", {
    method: "POST",
    cookieJar,
    json: printerInput,
  })
  const printer = printerResult.payload?.printer
  assert(printer?.id, "printer registration did not return printer id")
  return printer
}

async function main() {
  logStep(`waiting for server health at ${serverBaseUrl}`)
  const serverHealth = await waitForJson(
    "server health",
    "/v1/health",
    { baseUrl: serverBaseUrl },
    (payload) => payload?.status === "ok",
  )
  assert(serverHealth.database_driver === "sqlite", "server health should report sqlite driver")

  logStep(`waiting for web proxy health at ${webBaseUrl}`)
  const webHealth = await waitForJson(
    "web health",
    "/v1/health",
    { baseUrl: webBaseUrl },
    (payload) => payload?.status === "ok",
  )
  assert(webHealth.database_driver === "sqlite", "web health should report sqlite driver")

  const cookieJar = new CookieJar()
  logStep("logging in through web proxy")
  const login = await loginWithFallback(cookieJar)
  let currentPassword = login.password
  let authUser = login.response.user

  if (authUser?.must_change_password) {
    logStep("changing bootstrap admin password")
    const changed = await request("/v1/auth/change-password", {
      method: "POST",
      cookieJar,
      json: {
        current_password: currentPassword,
        new_password: rotatedPassword,
      },
    })
    currentPassword = rotatedPassword
    authUser = changed.payload.user
  }

  const me = await request("/v1/auth/me", { cookieJar })
  assert(me.payload?.authenticated === true, "session auth was not established")
  assert(
    me.payload?.user?.must_change_password === false,
    "admin password should already be rotated for smoke flow",
  )

  logStep("checking authenticated deep health")
  const deepHealth = await request("/v1/health/deep", { cookieJar })
  assert(deepHealth.payload?.db?.ok === true, "database deep health probe failed")
  assert(
    deepHealth.payload?.renderer_subprocess?.ok === true,
    "renderer subprocess deep health probe failed",
  )

  logStep("checking external CUPS reachability")
  const cupsDiscovery = await request("/v1/printers/discover/cups", { cookieJar })
  assert(Array.isArray(cupsDiscovery.payload?.printers), "CUPS discovery response is invalid")
  const candidate = selectDiscoveredPrinter(cupsDiscovery.payload.printers)
  assert(
    candidate,
    `no discovered CUPS printer matched "${preferredPrinterName}"`,
  )
  logStep(
    `registering discovered CUPS printer for print-path smoke: ${candidate.display_name} (${candidate.candidate_uri})`,
  )
  const managedPrinter = await registerManagedPrinter(cookieJar, {
    source: candidate.source || "cups_import",
    printer_uri: candidate.candidate_uri,
    display_name: candidate.display_name || preferredPrinterName,
  })
  const printerId = managedPrinter.id

  logStep("creating smoke template group and template")
  const groupResult = await request("/v1/templates/groups/create", {
    method: "POST",
    cookieJar,
    json: { name: templateGroupName },
  })
  const groupId = groupResult.payload?.group?.id
  assert(groupId, "template group creation did not return group id")

  const sampleData = {
    name: "张三",
    orderNo: `ORDER-${runId}`,
    note: "容器验收",
  }

  const templateCode = [
    "#set page(width: 80mm, height: auto, margin: 6mm)",
    "#set text(font: \"Noto Sans CJK SC\", size: 11pt)",
    "= DeepPrint 容器验收",
    "姓名: #data.name",
    "单号: #data.orderNo",
    "备注: #data.note",
  ].join("\n")

  const templateResult = await request("/v1/templates/create", {
    method: "POST",
    cookieJar,
    json: {
      group_id: groupId,
      name: templateName,
      description: "Container smoke validation template",
      output_name: "compose-smoke.pdf",
      typst_code: templateCode,
      sample_data: JSON.stringify(sampleData, null, 2),
    },
  })
  const templateId = templateResult.payload?.template?.id
  assert(templateId, "template creation did not return template id")

  logStep("creating open API key")
  const apiKeyResult = await request("/v1/api-keys/create", {
    method: "POST",
    cookieJar,
    json: {
      name: apiKeyName,
      scopes: ["template:read", "preview:create", "print:create", "job:read"],
    },
  })
  const apiToken = apiKeyResult.payload?.token
  const apiKeyId = apiKeyResult.payload?.api_key?.id
  assert(apiToken, "api key token missing from create response")
  assert(apiKeyId, "api key id missing from create response")

  logStep("verifying open template listing")
  const openTemplates = await request("/v1/open/templates", {
    headers: { authorization: `Bearer ${apiToken}` },
  })
  const templateFound = openTemplates.payload?.groups?.some((group) =>
    group.templates?.some((template) => template.id === templateId),
  )
  assert(templateFound, "new template was not visible from open templates API")

  logStep("rendering open preview PDF")
  const previewPdf = await request("/v1/open/preview", {
    method: "POST",
    expectBinary: true,
    headers: { authorization: `Bearer ${apiToken}` },
    json: {
      template_id: templateId,
      data: sampleData,
      print_options: {},
    },
  })
  const previewType = previewPdf.headers.get("content-type") || ""
  assert(previewType.includes("application/pdf"), "open preview did not return a PDF")
  assert(
    previewPdf.payload.subarray(0, 4).toString("utf8") === "%PDF",
    "open preview payload is not a PDF file",
  )

  logStep("creating open print job")
  const openPrint = await request("/v1/open/print", {
    method: "POST",
    headers: { authorization: `Bearer ${apiToken}` },
    json: {
      request_id: openRequestId,
      template_id: templateId,
      printer_id: printerId,
      data: sampleData,
      print_options: {},
    },
  })
  const openJobId = openPrint.payload?.job_id
  assert(openJobId, "open print did not return job id")

  const openJob = await pollOpenJob(apiToken, openJobId)
  assert(openJob.status === "succeeded", "open print job did not succeed")

  logStep("verifying session-side job query")
  const directJob = await request(`/v1/jobs/${encodeURIComponent(openJobId)}`, {
    cookieJar,
  })
  assert(directJob.payload?.status === "succeeded", "session job query did not report success")

  logStep("revoking API key")
  await request(`/v1/api-keys/${encodeURIComponent(apiKeyId)}/revoke`, {
    method: "POST",
    cookieJar,
  })

  const revokedAttempt = await request("/v1/open/templates", {
    headers: { authorization: `Bearer ${apiToken}` },
    throwOnHttpError: false,
  })
  assert(
    revokedAttempt.status === 401,
    `revoked API key should be rejected, got HTTP ${revokedAttempt.status}`,
  )

  logStep("logging out")
  await request("/v1/auth/logout", {
    method: "POST",
    cookieJar,
  })

  const logoutCheck = await request("/v1/auth/me", {
    cookieJar,
    throwOnHttpError: false,
  })
  assert(
    (logoutCheck.status === 200 && logoutCheck.payload?.authenticated === false) ||
      logoutCheck.status === 401,
    `logout check expected anonymous auth/me state, got HTTP ${logoutCheck.status}`,
  )

  console.log("")
  console.log("[compose-smoke] success")
  console.log(`[compose-smoke] server=${serverBaseUrl}`)
  console.log(`[compose-smoke] web=${webBaseUrl}`)
  console.log(`[compose-smoke] admin=${adminUsername}`)
  console.log(`[compose-smoke] current_admin_password=${currentPassword}`)
  console.log(`[compose-smoke] printer_id=${printerId}`)
  console.log(`[compose-smoke] template_id=${templateId}`)
  console.log(`[compose-smoke] job_id=${openJobId}`)
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error)
  console.error(`[compose-smoke] error: ${message}`)
  process.exitCode = 1
})
