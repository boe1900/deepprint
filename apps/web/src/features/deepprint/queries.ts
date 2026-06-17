import type { QueryKey } from "@tanstack/react-query";
import { requestJson } from "./api";
import type {
  DeepHealthResponse,
  HealthResponse,
  JobResponse,
  JobsListResponse,
  PrinterDetail,
  PrintersResponse,
  RecentJobsResponse,
  TemplateWorkspaceResponse,
  TypstFontsResponse,
  TypstPackagesResponse,
} from "./types";

export const deepprintQueryKeys = {
  root: ["deepprint"] as const,
  agent: (baseUrl: string) =>
    [...deepprintQueryKeys.root, "agent", baseUrl] as const,
  health: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "health"] as const,
  deepHealth: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "deep-health"] as const,
  printers: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "printers"] as const,
  printerDetail: (baseUrl: string, printerId: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "printer-detail", printerId] as const,
  jobs: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "jobs"] as const,
  jobDetail: (baseUrl: string, jobId: string) =>
    [...deepprintQueryKeys.jobs(baseUrl), "detail", jobId] as const,
  jobsList: (
    baseUrl: string,
    page: number,
    pageSize: number,
    status: string | null,
    printerId: string | null,
    search: string | null,
  ) =>
    [
      ...deepprintQueryKeys.jobs(baseUrl),
      "list",
      { page, pageSize, status, printerId, search },
    ] as const,
  recentJobs: (baseUrl: string, printerId: string | null, limit: number) =>
    [...deepprintQueryKeys.agent(baseUrl), "recent-jobs", { printerId, limit }] as const,
  templates: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "templates"] as const,
  templateWorkspace: (baseUrl: string) =>
    [...deepprintQueryKeys.templates(baseUrl), "workspace"] as const,
  typst: (baseUrl: string) =>
    [...deepprintQueryKeys.agent(baseUrl), "typst"] as const,
  typstPackages: (baseUrl: string) =>
    [...deepprintQueryKeys.typst(baseUrl), "packages"] as const,
  typstFonts: (baseUrl: string) =>
    [...deepprintQueryKeys.typst(baseUrl), "fonts"] as const,
};

interface ReadQueryOptionsArgs {
  baseUrl: string;
  path: string;
  queryKey: QueryKey;
  timeoutMs: number;
}

function createReadQueryOptions<T>({
  baseUrl,
  path,
  queryKey,
  timeoutMs,
}: ReadQueryOptionsArgs) {
  return {
    queryKey,
    queryFn: ({ signal }: { signal?: AbortSignal }) =>
      requestJson<T>(baseUrl, path, { signal, timeoutMs }),
  };
}

export function createHealthQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<HealthResponse>({
    baseUrl,
    path: "/v1/health",
    queryKey: deepprintQueryKeys.health(baseUrl),
    timeoutMs,
  });
}

export function createDeepHealthQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<DeepHealthResponse>({
    baseUrl,
    path: "/v1/health/deep",
    queryKey: deepprintQueryKeys.deepHealth(baseUrl),
    timeoutMs,
  });
}

export function createPrintersQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<PrintersResponse>({
    baseUrl,
    path: "/v1/printers",
    queryKey: deepprintQueryKeys.printers(baseUrl),
    timeoutMs,
  });
}

export function createPrinterDetailQueryOptions({
  baseUrl,
  printerId,
  timeoutMs,
}: {
  baseUrl: string;
  printerId: string;
  timeoutMs: number;
}) {
  return createReadQueryOptions<PrinterDetail>({
    baseUrl,
    path: `/v1/printers/${encodeURIComponent(printerId)}`,
    queryKey: deepprintQueryKeys.printerDetail(baseUrl, printerId),
    timeoutMs,
  });
}

export function createJobDetailQueryOptions({
  baseUrl,
  jobId,
  timeoutMs,
}: {
  baseUrl: string;
  jobId: string;
  timeoutMs: number;
}) {
  return createReadQueryOptions<JobResponse>({
    baseUrl,
    path: `/v1/jobs/${encodeURIComponent(jobId)}`,
    queryKey: deepprintQueryKeys.jobDetail(baseUrl, jobId),
    timeoutMs,
  });
}

export function createJobsListQueryOptions({
  baseUrl,
  page,
  pageSize,
  status,
  printerId,
  search,
  timeoutMs,
}: {
  baseUrl: string;
  page: number;
  pageSize: number;
  status: string | null;
  printerId: string | null;
  search?: string | null;
  timeoutMs: number;
}) {
  const params = new URLSearchParams({
    page: String(page),
    page_size: String(pageSize),
  });
  if (status) params.set("status", status);
  if (printerId) params.set("printer_id", printerId);
  if (search?.trim()) params.set("q", search.trim());

  return createReadQueryOptions<JobsListResponse>({
    baseUrl,
    path: `/v1/jobs?${params.toString()}`,
    queryKey: deepprintQueryKeys.jobsList(
      baseUrl,
      page,
      pageSize,
      status,
      printerId,
      search?.trim() || null,
    ),
    timeoutMs,
  });
}

export function createRecentJobsQueryOptions({
  baseUrl,
  limit,
  printerId,
  timeoutMs,
}: {
  baseUrl: string;
  limit: number;
  printerId: string | null;
  timeoutMs: number;
}) {
  const params = new URLSearchParams({ limit: String(limit) });
  if (printerId) params.set("printer_id", printerId);

  return createReadQueryOptions<RecentJobsResponse>({
    baseUrl,
    path: `/v1/jobs/recent?${params.toString()}`,
    queryKey: deepprintQueryKeys.recentJobs(baseUrl, printerId, limit),
    timeoutMs,
  });
}

export function createTemplateWorkspaceQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<TemplateWorkspaceResponse>({
    baseUrl,
    path: "/v1/templates/workspace",
    queryKey: deepprintQueryKeys.templateWorkspace(baseUrl),
    timeoutMs,
  });
}

export function createTypstPackagesQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<TypstPackagesResponse>({
    baseUrl,
    path: "/v1/typst/packages",
    queryKey: deepprintQueryKeys.typstPackages(baseUrl),
    timeoutMs,
  });
}

export function createTypstFontsQueryOptions(baseUrl: string, timeoutMs: number) {
  return createReadQueryOptions<TypstFontsResponse>({
    baseUrl,
    path: "/v1/typst/fonts",
    queryKey: deepprintQueryKeys.typstFonts(baseUrl),
    timeoutMs,
  });
}
