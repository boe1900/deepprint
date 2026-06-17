import type { RequestTimeoutSettings } from "./types";

export const BASE_URL_STORAGE_KEY = "deepprint-studio.agent-base-url";
export const THEME_MODE_STORAGE_KEY = "deepprint-studio.theme-mode";
export const REQUEST_TIMEOUTS_STORAGE_KEY = "deepprint-studio.request-timeouts-ms";
export const DEFAULT_BASE_URL = "";
export const AUTO_REFRESH_INTERVAL_MS = 15000;
export const DEFAULT_JOB_POLL_INTERVAL_SEC = 3;
export const MIN_JOB_POLL_INTERVAL_SEC = 1;
export const MAX_JOB_POLL_INTERVAL_SEC = 60;
export const REQUEST_TIMEOUTS_MS: RequestTimeoutSettings = {
  health: 5000,
  deepHealth: 7000,
  printers: 8000,
  jobStatus: 8000,
  writes: 15000,
  diagnosticsExport: 30000,
  urlProbe: 5000,
};

export const DEFAULT_CREATE_TEMPLATE =
  "#set page(width: 80mm, height: auto)\n#set text(size: 10pt)\n*Order:* #data.orderNo\n*Buyer:* #data.buyer";

export const DEFAULT_CREATE_DATA_JSON = JSON.stringify(
  { orderNo: "A1001", buyer: "张三" },
  null,
  2,
);

export const DIAGNOSTICS_HISTORY_STORAGE_KEY = "deepprint-studio.diagnostics-history";
export const DIAGNOSTICS_HISTORY_MAX_ITEMS = 50;

export const JOB_STAGE_FLOW = ["queued", "rendering", "printing", "succeeded"] as const;
