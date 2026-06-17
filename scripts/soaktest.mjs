#!/usr/bin/env node

import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

if (process.argv.includes("--help") || process.argv.includes("-h")) {
  printHelp();
  process.exit(0);
}

const baseUrl =
  process.env.DEEPPRINT_SOAK_BASE_URL ||
  process.env.DEEPPRINT_SERVER_BASE_URL ||
  process.env.DEEPPRINT_AGENT_BASE_URL ||
  "http://127.0.0.1:17801";
const durationHours = parsePositiveFloat(process.env.DEEPPRINT_SOAK_DURATION_HOURS, 72);
const batchJobs = parsePositiveInt(process.env.DEEPPRINT_SOAK_BATCH_JOBS, 100);
const createConcurrency = parsePositiveInt(process.env.DEEPPRINT_SOAK_CREATE_CONCURRENCY, 10);
const pollConcurrency = parsePositiveInt(process.env.DEEPPRINT_SOAK_POLL_CONCURRENCY, 20);
const pollIntervalMs = parsePositiveInt(process.env.DEEPPRINT_SOAK_POLL_INTERVAL_MS, 600);
const batchTimeoutSec = parsePositiveInt(process.env.DEEPPRINT_SOAK_BATCH_TIMEOUT_SEC, 900);
const batchIntervalSec = parsePositiveInt(process.env.DEEPPRINT_SOAK_BATCH_INTERVAL_SEC, 5);
const printerName = (process.env.DEEPPRINT_SOAK_PRINTER_NAME || "").trim();
const stopOnFailure = parseBool(process.env.DEEPPRINT_SOAK_STOP_ON_FAILURE, false);
const exportDiagOnFailure = parseBool(process.env.DEEPPRINT_SOAK_EXPORT_DIAG_ON_FAILURE, true);
const runId = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
const reportPath =
  (process.env.DEEPPRINT_SOAK_REPORT_PATH || "").trim() ||
  path.join(os.tmpdir(), `deepprint-soak-${runId}.json`);
const durationMs = Math.max(1, Math.floor(durationHours * 3600 * 1000));
const terminalStatuses = new Set(["succeeded", "failed", "canceled"]);

const diagnosticRequest = {
  include_logs: parseBool(process.env.DEEPPRINT_SOAK_DIAG_INCLUDE_LOGS, true),
  log_max_files: parsePositiveInt(process.env.DEEPPRINT_SOAK_DIAG_LOG_MAX_FILES, 5),
  log_tail_lines: parsePositiveInt(process.env.DEEPPRINT_SOAK_DIAG_LOG_TAIL_LINES, 3000),
  log_max_bytes_per_file: parsePositiveInt(
    process.env.DEEPPRINT_SOAK_DIAG_LOG_MAX_BYTES_PER_FILE,
    512 * 1024,
  ),
  failed_jobs_limit: parsePositiveInt(process.env.DEEPPRINT_SOAK_DIAG_FAILED_JOBS_LIMIT, 200),
};

function printHelp() {
  console.log(`
DeepPrint soak test script.

Usage:
  bun run soaktest

Key env vars:
  DEEPPRINT_SOAK_BASE_URL=http://127.0.0.1:17801
  DEEPPRINT_SOAK_DURATION_HOURS=72
  DEEPPRINT_SOAK_BATCH_JOBS=100
  DEEPPRINT_SOAK_CREATE_CONCURRENCY=10
  DEEPPRINT_SOAK_POLL_CONCURRENCY=20
  DEEPPRINT_SOAK_BATCH_TIMEOUT_SEC=900
  DEEPPRINT_SOAK_STOP_ON_FAILURE=false
  DEEPPRINT_SOAK_EXPORT_DIAG_ON_FAILURE=true
  DEEPPRINT_SOAK_REPORT_PATH=/path/to/report.json

  `);
}

function parsePositiveInt(raw, fallback) {
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback;
  return parsed;
}

function parsePositiveFloat(raw, fallback) {
  if (!raw) return fallback;
  const parsed = Number.parseFloat(raw);
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback;
  return parsed;
}

function parseBool(raw, fallback) {
  if (!raw) return fallback;
  return ["1", "true", "yes", "on"].includes(raw.trim().toLowerCase());
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function nowIso() {
  return new Date().toISOString();
}

async function fetchJsonOrText(url, options = {}) {
  const response = await fetch(url, options);
  const text = await response.text();
  let payload;
  try {
    payload = text ? JSON.parse(text) : null;
  } catch {
    payload = text;
  }
  return { ok: response.ok, status: response.status, payload };
}

async function postJson(apiPath, payload) {
  const body = JSON.stringify(payload);
  const headers = { "content-type": "application/json" };

  const res = await fetchJsonOrText(`${baseUrl}${apiPath}`, {
    method: "POST",
    headers,
    body,
  });
  if (!res.ok) {
    throw new Error(
      `POST ${apiPath} failed: http=${res.status}, payload=${JSON.stringify(res.payload)}`,
    );
  }
  return res.payload;
}

async function getJson(apiPath) {
  const res = await fetchJsonOrText(`${baseUrl}${apiPath}`);
  if (!res.ok) {
    throw new Error(
      `GET ${apiPath} failed: http=${res.status}, payload=${JSON.stringify(res.payload)}`,
    );
  }
  return res.payload;
}

async function runWorkerPool(total, concurrency, workerFn) {
  let nextIndex = 0;
  const results = new Array(total);
  const workers = Array.from({ length: Math.min(total, concurrency) }, async () => {
    while (true) {
      const current = nextIndex;
      if (current >= total) return;
      nextIndex += 1;
      results[current] = await workerFn(current);
    }
  });
  await Promise.all(workers);
  return results;
}

async function mapWithConcurrency(items, concurrency, mapper) {
  if (items.length === 0) return [];
  let nextIndex = 0;
  const results = new Array(items.length);
  const workers = Array.from({ length: Math.min(items.length, concurrency) }, async () => {
    while (true) {
      const current = nextIndex;
      if (current >= items.length) return;
      nextIndex += 1;
      results[current] = await mapper(items[current], current);
    }
  });
  await Promise.all(workers);
  return results;
}

async function postJob(batchIndex, seq) {
  const payload = {
    request_id: `soak-${runId}-${batchIndex}-${seq}`,
    template_content:
      "#set page(width: 80mm, height: auto)\n#set text(size: 11pt)\n= DeepPrint Soak Test\nOrder: #data.orderNo\nName: #data.buyer\n",
    data: {
      orderNo: `SOAK-${batchIndex}-${seq}`,
      buyer: `buyer-${seq}`,
    },
    print_options: {
      copies: 1,
      ...(printerName ? { printer_name: printerName } : {}),
    },
  };

  const body = await postJson("/v1/jobs", payload, true);
  if (!body?.job_id) {
    throw new Error(`create job response missing job_id: ${JSON.stringify(body)}`);
  }
  return {
    jobId: body.job_id,
    idempotent: Boolean(body.idempotent),
  };
}

async function getJobStatus(jobId) {
  const body = await getJson(`/v1/jobs/${encodeURIComponent(jobId)}`);
  if (!body?.status) {
    throw new Error(
      `job response missing status: job_id=${jobId}, payload=${JSON.stringify(body)}`,
    );
  }
  return body.status;
}

async function runBatch(batchIndex) {
  const startedAt = Date.now();
  const batchDeadline = startedAt + batchTimeoutSec * 1000;
  const created = await runWorkerPool(batchJobs, createConcurrency, async (index) =>
    postJob(batchIndex, index + 1),
  );
  const idempotent = created.filter((it) => it.idempotent).length;
  const pending = new Map(created.map((it) => [it.jobId, "queued"]));
  const statusCounters = { succeeded: 0, failed: 0, canceled: 0 };
  let timedOut = false;

  while (pending.size > 0) {
    if (Date.now() > batchDeadline) {
      timedOut = true;
      break;
    }

    const ids = Array.from(pending.keys());
    const statuses = await mapWithConcurrency(ids, pollConcurrency, async (jobId) => ({
      jobId,
      status: await getJobStatus(jobId),
    }));

    for (const item of statuses) {
      if (!item) continue;
      pending.set(item.jobId, item.status);
      if (terminalStatuses.has(item.status)) {
        pending.delete(item.jobId);
        if (item.status in statusCounters) {
          statusCounters[item.status] += 1;
        }
      }
    }

    if (pending.size > 0) {
      await sleep(pollIntervalMs);
    }
  }

  const endedAt = Date.now();
  const elapsedSec = Math.max(0.001, (endedAt - startedAt) / 1000);
  return {
    batch_index: batchIndex,
    started_at: new Date(startedAt).toISOString(),
    ended_at: new Date(endedAt).toISOString(),
    elapsed_sec: Number(elapsedSec.toFixed(3)),
    created: created.length,
    idempotent,
    succeeded: statusCounters.succeeded,
    failed: statusCounters.failed,
    canceled: statusCounters.canceled,
    pending_on_timeout: pending.size,
    timed_out: timedOut,
    throughput_jobs_per_sec: Number((created.length / elapsedSec).toFixed(3)),
  };
}

async function tryExportDiagnostics(batchIndex, reason) {
  if (!exportDiagOnFailure) return null;
  try {
    const response = await postJson("/v1/diagnostics/export", diagnosticRequest, true);
    return {
      batch_index: batchIndex,
      reason,
      exported_at: nowIso(),
      response,
    };
  } catch (error) {
    return {
      batch_index: batchIndex,
      reason,
      exported_at: nowIso(),
      error: error?.message || String(error),
    };
  }
}

async function tryHealthSnapshot() {
  try {
    return await getJson("/v1/health");
  } catch (error) {
    return {
      error: error?.message || String(error),
    };
  }
}

async function writeReport(report) {
  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
}

async function main() {
  console.log(
    `[soaktest] start base=${baseUrl}, duration_h=${durationHours}, batch_jobs=${batchJobs}, create_concurrency=${createConcurrency}, poll_concurrency=${pollConcurrency}, stop_on_failure=${stopOnFailure}, auth=${Boolean(
      authToken && authSecret,
    )}`,
  );

  const startedAtMs = Date.now();
  const deadlineMs = startedAtMs + durationMs;
  const report = {
    run_id: runId,
    base_url: baseUrl,
    started_at: new Date(startedAtMs).toISOString(),
    planned_duration_hours: durationHours,
    config: {
      batch_jobs: batchJobs,
      create_concurrency: createConcurrency,
      poll_concurrency: pollConcurrency,
      poll_interval_ms: pollIntervalMs,
      batch_timeout_sec: batchTimeoutSec,
      batch_interval_sec: batchIntervalSec,
      stop_on_failure: stopOnFailure,
      export_diag_on_failure: exportDiagOnFailure,
      printer_name: printerName || null,
      auth_enabled: Boolean(authToken && authSecret),
    },
    health_start: await tryHealthSnapshot(),
    batches: [],
    failures: [],
    totals: {
      created: 0,
      succeeded: 0,
      failed: 0,
      canceled: 0,
      timed_out_batches: 0,
    },
    fatal_error: null,
    finished_at: null,
    elapsed_sec: 0,
  };

  try {
    let batchIndex = 1;
    while (Date.now() < deadlineMs) {
      const batch = await runBatch(batchIndex);
      report.batches.push(batch);
      report.totals.created += batch.created;
      report.totals.succeeded += batch.succeeded;
      report.totals.failed += batch.failed;
      report.totals.canceled += batch.canceled;
      if (batch.timed_out) {
        report.totals.timed_out_batches += 1;
      }

      console.log(
        `[soaktest] batch=${batchIndex} created=${batch.created} succeeded=${batch.succeeded} failed=${batch.failed} canceled=${batch.canceled} timeout_pending=${batch.pending_on_timeout} throughput=${batch.throughput_jobs_per_sec}/s`,
      );

      const hasFailure = batch.failed > 0 || batch.canceled > 0 || batch.pending_on_timeout > 0;
      if (hasFailure) {
        const reason = batch.timed_out ? "batch_timeout" : "terminal_failure";
        const diag = await tryExportDiagnostics(batchIndex, reason);
        report.failures.push({
          batch_index: batchIndex,
          reason,
          batch,
          diagnostics: diag,
        });
        if (stopOnFailure) {
          break;
        }
      }

      batchIndex += 1;
      if (Date.now() < deadlineMs && batchIntervalSec > 0) {
        await sleep(batchIntervalSec * 1000);
      }
    }
  } catch (error) {
    report.fatal_error = error?.message || String(error);
  }

  report.health_end = await tryHealthSnapshot();
  report.finished_at = nowIso();
  report.elapsed_sec = Math.max(0, Math.floor((Date.now() - startedAtMs) / 1000));
  await writeReport(report);

  console.log(
    `[soaktest] done batches=${report.batches.length}, created=${report.totals.created}, succeeded=${report.totals.succeeded}, failed=${report.totals.failed}, canceled=${report.totals.canceled}, timed_out_batches=${report.totals.timed_out_batches}`,
  );
  console.log(`[soaktest] report=${reportPath}`);

  if (report.fatal_error) {
    throw new Error(report.fatal_error);
  }
  if (report.failures.length > 0) {
    process.exitCode = 2;
  }
}

main().catch((error) => {
  console.error(`[soaktest] error: ${error?.message || error}`);
  process.exitCode = 1;
});
