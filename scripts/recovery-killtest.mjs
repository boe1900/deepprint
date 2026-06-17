#!/usr/bin/env node

import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const port = parsePositiveInt(
  process.env.DEEPPRINT_RECOVERY_SERVER_PORT || process.env.DEEPPRINT_RECOVERY_AGENT_PORT,
  17820,
);
const baseUrl = `http://127.0.0.1:${port}`;
const healthTimeoutSec = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_HEALTH_TIMEOUT_SEC, 60);
const stageTimeoutSec = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_STAGE_TIMEOUT_SEC, 40);
const finalTimeoutSec = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_FINAL_TIMEOUT_SEC, 120);
const pollIntervalMs = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_POLL_INTERVAL_MS, 200);
const renderEngine = (process.env.DEEPPRINT_RECOVERY_RENDER_ENGINE || "text").trim() || "text";
const workerPollMs = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_WORKER_POLL_MS, 100);
const backendPollMs = parsePositiveInt(process.env.DEEPPRINT_RECOVERY_BACKEND_POLL_MS, 200);
const keepData = parseBool(process.env.DEEPPRINT_RECOVERY_KEEP_DATA, false);
const runId = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
const dataDir =
  (process.env.DEEPPRINT_RECOVERY_DATA_DIR || "").trim() ||
  path.join(os.tmpdir(), `deepprint-recovery-${runId}`);

const binaryName = process.platform === "win32" ? "deepprint-server.exe" : "deepprint-server";
const binaryPath = path.resolve("apps/server", "target", "debug", binaryName);
const terminalStatuses = new Set(["succeeded", "failed", "canceled"]);

function parsePositiveInt(raw, fallback) {
  if (!raw) return fallback;
  const value = Number.parseInt(raw, 10);
  if (!Number.isFinite(value) || value <= 0) return fallback;
  return value;
}

function parseBool(raw, fallback) {
  if (!raw) return fallback;
  return ["1", "true", "yes", "on"].includes(raw.trim().toLowerCase());
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function attachOutput(label, child) {
  child.stdout?.on("data", (chunk) => {
    const text = chunk.toString().trim();
    if (!text) return;
    for (const line of text.split(/\r?\n/)) {
      if (line.trim()) console.log(`[${label}] ${line}`);
    }
  });

  child.stderr?.on("data", (chunk) => {
    const text = chunk.toString().trim();
    if (!text) return;
    for (const line of text.split(/\r?\n/)) {
      if (line.trim()) console.error(`[${label}] ${line}`);
    }
  });
}

function waitForExit(child, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    if (child.exitCode !== null) {
      resolve({ code: child.exitCode, signal: child.signalCode });
      return;
    }

    const timer = setTimeout(() => {
      reject(new Error(`process ${child.pid} exit timeout`));
    }, timeoutMs);

    child.once("exit", (code, signal) => {
      clearTimeout(timer);
      resolve({ code, signal });
    });
  });
}

async function runCommand(cmd, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      stdio: "inherit",
      env: process.env,
    });
    child.once("error", reject);
    child.once("exit", (code) => {
      if (code === 0) {
        resolve(undefined);
      } else {
        reject(new Error(`${cmd} exited with code ${code}`));
      }
    });
  });
}

function startServer(label) {
  const env = {
    ...process.env,
    DEEPPRINT_AGENT_BIND: "127.0.0.1",
    DEEPPRINT_AGENT_PORT: String(port),
    DEEPPRINT_AGENT_MOCK: "true",
    DEEPPRINT_AGENT_DATA_DIR: dataDir,
    DEEPPRINT_AGENT_WORKER_CONCURRENCY: "1",
    DEEPPRINT_AGENT_WORKER_POLL_MS: String(workerPollMs),
    DEEPPRINT_BACKEND_STATUS_POLL_MS: String(backendPollMs),
    DEEPPRINT_BACKEND_STATUS_TIMEOUT_SEC: "20",
    DEEPPRINT_RENDER_TIMEOUT_SEC: "40",
    DEEPPRINT_RENDER_ENGINE: renderEngine,
  };

  const child = spawn(binaryPath, [], {
    env,
    stdio: ["ignore", "pipe", "pipe"],
  });
  attachOutput(label, child);
  return child;
}

async function forceKill(child) {
  if (child.exitCode !== null) {
    return;
  }

  if (process.platform === "win32") {
    await runCommand("taskkill", ["/PID", String(child.pid), "/T", "/F"]);
    return;
  }

  child.kill("SIGKILL");
  await waitForExit(child, 10000);
}

async function stopGracefully(child) {
  if (child.exitCode !== null) {
    return;
  }

  if (process.platform === "win32") {
    await runCommand("taskkill", ["/PID", String(child.pid), "/T", "/F"]);
    return;
  }

  child.kill("SIGTERM");
  try {
    await waitForExit(child, 8000);
  } catch {
    await forceKill(child);
  }
}

async function fetchJson(url, options) {
  const response = await fetch(url, options);
  const text = await response.text();
  let payload = null;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    payload = text;
  }
  return { ok: response.ok, status: response.status, payload };
}

async function ensurePrinter() {
  const payload = {
    source: "manual",
    printer_uri: "mock:printer",
    display_name: "Recovery Test Printer",
  };

  const res = await fetchJson(`${baseUrl}/v1/printers`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!res.ok || !res.payload?.printer?.id) {
    throw new Error(
      `create printer failed: http=${res.status}, payload=${JSON.stringify(res.payload)}`,
    );
  }

  return res.payload.printer.id;
}

async function waitForHealth() {
  const deadline = Date.now() + healthTimeoutSec * 1000;
  while (Date.now() < deadline) {
    try {
      const res = await fetchJson(`${baseUrl}/v1/health`);
      if (res.ok) return;
    } catch {
      // server not ready yet
    }
    await sleep(300);
  }
  throw new Error(`server health timeout after ${healthTimeoutSec}s`);
}

async function createJob(printerId) {
  const requestId = `recovery-${runId}`;
  const payload = {
    request_id: requestId,
    printer_id: printerId,
    template_content:
      "#set page(width: 80mm, height: auto)\n#set text(size: 12pt)\n= Recovery Test\nOrder: #data.orderNo\n",
    data: {
      orderNo: `ORDER-${runId}`,
    },
    print_options: {
      copies: 1,
    },
  };

  const res = await fetchJson(`${baseUrl}/v1/jobs`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!res.ok || !res.payload?.job_id) {
    throw new Error(
      `create job failed: http=${res.status}, payload=${JSON.stringify(res.payload)}`,
    );
  }

  return res.payload.job_id;
}

async function getJobStatus(jobId) {
  const res = await fetchJson(`${baseUrl}/v1/jobs/${encodeURIComponent(jobId)}`);
  if (!res.ok || !res.payload?.status) {
    throw new Error(`query job failed: http=${res.status}, payload=${JSON.stringify(res.payload)}`);
  }
  return res.payload.status;
}

async function waitForAnyStatus(jobId, expectedStatuses, timeoutSec) {
  const deadline = Date.now() + timeoutSec * 1000;
  let lastStatus = "unknown";

  while (Date.now() < deadline) {
    let status = null;
    try {
      status = await getJobStatus(jobId);
    } catch (err) {
      lastStatus = `query-error:${err?.message || err}`;
      await sleep(pollIntervalMs);
      continue;
    }

    lastStatus = status;
    if (expectedStatuses.has(status)) {
      return status;
    }
    if (terminalStatuses.has(status) && !expectedStatuses.has(status)) {
      throw new Error(`job reached terminal status early: ${status}`);
    }

    await sleep(pollIntervalMs);
  }

  throw new Error(`status wait timeout after ${timeoutSec}s, last_status=${lastStatus}`);
}

async function main() {
  let firstServer = null;
  let secondServer = null;

  try {
    await fs.mkdir(dataDir, { recursive: true });
    console.log(`[recovery] data_dir=${dataDir}`);
    console.log(`[recovery] building deepprint-server binary`);
    await runCommand("cargo", [
      "build",
      "--manifest-path",
      "apps/server/Cargo.toml",
    ]);

    console.log(`[recovery] phase1 start`);
    firstServer = startServer("server-1");
    await waitForHealth();
    const printerId = await ensurePrinter();
    console.log(`[recovery] printer_id=${printerId}`);
    const jobId = await createJob(printerId);
    console.log(`[recovery] created job_id=${jobId}`);

    const stageStatus = await waitForAnyStatus(
      jobId,
      new Set(["rendering", "printing"]),
      stageTimeoutSec,
    );
    console.log(`[recovery] reached status=${stageStatus}, force killing server`);
    await forceKill(firstServer);
    firstServer = null;

    console.log(`[recovery] phase2 restart`);
    secondServer = startServer("server-2");
    await waitForHealth();

    const finalStatus = await waitForAnyStatus(jobId, new Set(["succeeded"]), finalTimeoutSec);
    console.log(`[recovery] final status=${finalStatus}`);
    console.log("[recovery] pass");
  } finally {
    if (firstServer) {
      await stopGracefully(firstServer);
    }
    if (secondServer) {
      await stopGracefully(secondServer);
    }
    if (!keepData) {
      await fs.rm(dataDir, { recursive: true, force: true });
    }
  }
}

main().catch((err) => {
  console.error(`[recovery] fail: ${err?.message || err}`);
  process.exitCode = 1;
});
