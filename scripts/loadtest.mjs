#!/usr/bin/env node

const baseUrl =
  process.env.DEEPPRINT_SERVER_BASE_URL ||
  process.env.DEEPPRINT_AGENT_BASE_URL ||
  "http://127.0.0.1:17801";
const totalJobs = parsePositiveInt(process.env.DEEPPRINT_LOADTEST_JOBS, 1000);
const createConcurrency = parsePositiveInt(process.env.DEEPPRINT_LOADTEST_CREATE_CONCURRENCY, 20);
const pollConcurrency = parsePositiveInt(process.env.DEEPPRINT_LOADTEST_POLL_CONCURRENCY, 40);
const pollIntervalMs = parsePositiveInt(process.env.DEEPPRINT_LOADTEST_POLL_INTERVAL_MS, 600);
const timeoutSec = parsePositiveInt(process.env.DEEPPRINT_LOADTEST_TIMEOUT_SEC, 600);
const printerName = (process.env.DEEPPRINT_LOADTEST_PRINTER_NAME || "").trim();

const runId = `${Date.now()}-${Math.floor(Math.random() * 10000)}`;
const startedAt = Date.now();

const terminalStatuses = new Set(["succeeded", "failed", "canceled"]);

function parsePositiveInt(raw, fallback) {
  if (!raw) return fallback;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback;
  return parsed;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function postJob(requestId, seq) {
  const payload = {
    request_id: requestId,
    template_content:
      "#set page(width: 80mm, height: auto)\n#set text(size: 11pt)\n= DeepPrint Load Test\nOrder: #data.orderNo\nName: #data.buyer\n",
    data: {
      orderNo: `ORDER-${seq}`,
      buyer: `buyer-${seq}`,
    },
    print_options: {
      copies: 1,
      ...(printerName ? { printer_name: printerName } : {}),
    },
  };

  const response = await fetch(`${baseUrl}/v1/jobs`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`create job failed: http=${response.status}, body=${text}`);
  }

  const body = await response.json();
  if (!body?.job_id) {
    throw new Error(`create job response missing job_id: ${JSON.stringify(body)}`);
  }

  return {
    requestId,
    jobId: body.job_id,
    idempotent: Boolean(body.idempotent),
    acceptedStatusCode: response.status,
  };
}

async function getJobStatus(jobId) {
  const response = await fetch(`${baseUrl}/v1/jobs/${encodeURIComponent(jobId)}`);
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`query job failed: job_id=${jobId}, http=${response.status}, body=${text}`);
  }
  const body = await response.json();
  return body.status;
}

async function runWorkerPool(total, concurrency, workerFn) {
  let nextIndex = 0;
  const results = [];

  const workers = Array.from({ length: Math.min(total, concurrency) }, async () => {
    while (true) {
      const current = nextIndex;
      if (current >= total) {
        return;
      }
      nextIndex += 1;
      const value = await workerFn(current);
      results.push(value);
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
      if (current >= items.length) {
        return;
      }
      nextIndex += 1;
      results[current] = await mapper(items[current], current);
    }
  });

  await Promise.all(workers);
  return results;
}

async function main() {
  console.log(
    `[loadtest] start base=${baseUrl}, jobs=${totalJobs}, create_concurrency=${createConcurrency}, poll_concurrency=${pollConcurrency}, poll_interval_ms=${pollIntervalMs}, timeout_sec=${timeoutSec}`,
  );

  const createResults = await runWorkerPool(totalJobs, createConcurrency, async (index) => {
    const requestId = `load-${runId}-${index + 1}`;
    return postJob(requestId, index + 1);
  });

  const accepted202 = createResults.filter((it) => it.acceptedStatusCode === 202).length;
  const accepted200 = createResults.filter((it) => it.acceptedStatusCode === 200).length;
  const idempotentCount = createResults.filter((it) => it.idempotent).length;
  const pending = new Map(createResults.map((it) => [it.jobId, "queued"]));
  const statusCounters = {
    succeeded: 0,
    failed: 0,
    canceled: 0,
  };

  while (pending.size > 0) {
    if ((Date.now() - startedAt) / 1000 > timeoutSec) {
      throw new Error(
        `loadtest timeout: pending=${pending.size}, elapsed_sec=${Math.floor((Date.now() - startedAt) / 1000)}`,
      );
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

  const elapsedSec = (Date.now() - startedAt) / 1000;
  const throughput = totalJobs / elapsedSec;
  console.log("[loadtest] done");
  console.log(
    `[loadtest] created=${createResults.length}, http202=${accepted202}, http200=${accepted200}, idempotent=${idempotentCount}`,
  );
  console.log(
    `[loadtest] terminal succeeded=${statusCounters.succeeded}, failed=${statusCounters.failed}, canceled=${statusCounters.canceled}`,
  );
  console.log(
    `[loadtest] elapsed_sec=${elapsedSec.toFixed(2)}, throughput_jobs_per_sec=${throughput.toFixed(2)}`,
  );

  if (statusCounters.failed > 0 || statusCounters.canceled > 0) {
    process.exitCode = 2;
  }
}

main().catch((err) => {
  console.error(`[loadtest] error: ${err?.message || err}`);
  process.exitCode = 1;
});
