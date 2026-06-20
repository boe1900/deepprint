import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircleIcon,
  CheckCircle2Icon,
  ClockIcon,
  FileTextIcon,
  InboxIcon,
  Loader2Icon,
  MoreHorizontalIcon,
  PrinterIcon,
  RefreshCwIcon,
  SearchIcon,
  XCircleIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { DeepprintController } from "@/features/deepprint/controller";
import { createJobsListQueryOptions, deepprintQueryKeys } from "@/features/deepprint/queries";
import type { JobResponse } from "@/features/deepprint/types";
import { formatUnixSec, statusLabel } from "@/features/deepprint/utils";
import { cn } from "@/lib/utils";
import { getCurrentLocale, translate, useI18n, type MessageKey } from "@/i18n";

type JobStatus =
  | "needs_attention"
  | "queued"
  | "rendering"
  | "submitting"
  | "printing"
  | "succeeded"
  | "failed"
  | "canceled";

type JobStatusTab = "needs_attention" | "active" | "finished" | "all";

const PAGE_SIZE = 10;
const ALL_JOB_STATUSES: JobStatus[] = [
  "needs_attention",
  "queued",
  "rendering",
  "submitting",
  "printing",
  "succeeded",
  "failed",
  "canceled",
];
const STATUS_TABS: Array<{
  value: JobStatusTab;
  labelKey: MessageKey;
  statuses: JobStatus[];
}> = [
  { value: "needs_attention", labelKey: "status.needs_attention", statuses: ["needs_attention"] },
  { value: "active", labelKey: "history.active", statuses: ["queued", "rendering", "submitting", "printing"] },
  { value: "finished", labelKey: "common.finished", statuses: ["succeeded", "failed", "canceled"] },
  { value: "all", labelKey: "common.all", statuses: ALL_JOB_STATUSES },
];
const ACTIVE_JOB_STATUSES = new Set(["queued", "rendering", "submitting", "printing"]);

export function PrintHistoryPage({
  controller,
  showHeader = true,
}: {
  controller: DeepprintController;
  showHeader?: boolean;
}) {
  const { t } = useI18n();
  const { actions, agent, ui, writes } = controller;
  const queryClient = useQueryClient();
  const [page, setPage] = useState(1);
  const [cancelingJobId, setCancelingJobId] = useState<string | null>(null);
  const [statusTab, setStatusTab] = useState<JobStatusTab>("needs_attention");
  const [printerFilter, setPrinterFilter] = useState("all");
  const [search, setSearch] = useState("");
  const statusParam = getStatusParam(statusTab);
  const printerId = printerFilter === "all" ? null : printerFilter;
  const normalizedSearch = search.trim() || null;
  const jobsQuery = useQuery({
    ...createJobsListQueryOptions({
      baseUrl: ui.baseUrl,
      page,
      pageSize: PAGE_SIZE,
      status: statusParam,
      printerId,
      search: normalizedSearch,
      timeoutMs: ui.requestTimeouts.jobStatus,
    }),
  });
  const jobs = jobsQuery.data?.jobs ?? [];
  const total = jobsQuery.data?.total ?? 0;
  const totalPages = jobsQuery.data?.total_pages ?? 1;
  const loading = jobsQuery.isFetching;
  const activeTab = STATUS_TABS.find((tab) => tab.value === statusTab) ?? STATUS_TABS[0];
  const activeJobsCount = agent.health
    ? (agent.health.rendering_jobs ?? 0) +
      (agent.health.submitting_jobs ?? 0) +
      (agent.health.printing_jobs ?? 0)
    : null;

  const printerOptions = useMemo(
    () => [
      { value: "all", label: t("history.printerAll") },
      ...agent.printers.map((printer) => ({
        value: printer.id,
        label: printer.name,
      })),
    ],
    [agent.printers, t],
  );
  const selectedPrinterLabel =
    printerOptions.find((option) => option.value === printerFilter)?.label ??
    (printerFilter === "all" ? t("history.printerAll") : t("history.printerSelected"));

  const loadJobs = async (showNotice = false) => {
    try {
      await queryClient.fetchQuery({
        ...createJobsListQueryOptions({
          baseUrl: ui.baseUrl,
          page,
          pageSize: PAGE_SIZE,
          status: statusParam,
          printerId,
          search: normalizedSearch,
          timeoutMs: ui.requestTimeouts.jobStatus,
        }),
        staleTime: 0,
      });
      if (showNotice) {
        ui.setNotice({ kind: "ok", message: t("history.refreshed") });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : t("history.loadFailed");
      ui.setNotice({ kind: "error", message });
    }
  };

  useEffect(() => {
    void actions.loadPrinters();
  }, [actions, ui.baseUrl]);

  const cancelJob = async (job: JobResponse) => {
    setCancelingJobId(job.job_id);
    try {
      writes.setCancelTargetJobId(job.job_id);
      await actions.onCancelJob({
        preventDefault: () => undefined,
      } as React.FormEvent<HTMLFormElement>);
      await queryClient.invalidateQueries({
        queryKey: deepprintQueryKeys.jobs(ui.baseUrl),
      });
      await loadJobs();
    } finally {
      setCancelingJobId(null);
    }
  };

  return (
    <div className="animate-in space-y-5 duration-300 fade-in slide-in-from-bottom-2">
      {showHeader ? (
        <SectionHeader
          activeJobsCount={activeJobsCount}
          needsAttentionCount={agent.health?.needs_attention_jobs ?? null}
        />
      ) : null}

      <Card className="gap-0 overflow-hidden py-0">
        <CardHeader className="border-b bg-muted/20 px-4 py-4 sm:px-5">
          <div className="min-w-0">
            <CardTitle>{t("history.title")}</CardTitle>
            <CardDescription>
              {t("history.description", { total })}
            </CardDescription>
          </div>
          <CardAction>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={loading}
              onClick={() => void loadJobs(true)}
            >
              <RefreshCwIcon
                data-icon="inline-start"
                className={loading ? "animate-spin" : undefined}
              />
              {t("common.refresh")}
            </Button>
          </CardAction>
        </CardHeader>

        <CardContent className="space-y-0 px-0">
          <div className="border-b px-4 py-4 sm:px-5">
            <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
              <Tabs
                value={statusTab}
                onValueChange={(value) => {
                  setPage(1);
                  setStatusTab(value as JobStatusTab);
                }}
              >
                <TabsList className="grid h-auto w-full grid-cols-2 sm:inline-flex sm:w-auto">
                  {STATUS_TABS.map((tab) => (
                    <TabsTrigger key={tab.value} value={tab.value} className="px-3 py-1.5">
                      {t(tab.labelKey)}
                      {tab.value === "needs_attention" && (agent.health?.needs_attention_jobs ?? 0) > 0 ? (
                        <Badge variant="destructive" className="ml-1 h-4 px-1.5 text-[10px]">
                          {agent.health?.needs_attention_jobs}
                        </Badge>
                      ) : null}
                    </TabsTrigger>
                  ))}
                </TabsList>
              </Tabs>

              <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_220px] lg:w-[520px]">
                <div className="relative">
                  <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    value={search}
                    onChange={(event) => {
                      setPage(1);
                      setSearch(event.target.value);
                    }}
                    className="pl-8"
                    placeholder={t("history.searchPlaceholder")}
                  />
                </div>
                <Select
                  value={printerFilter}
                  onValueChange={(value) => {
                    setPage(1);
                    setPrinterFilter(value ?? "all");
                  }}
                >
                  <SelectTrigger className="w-full">
                    <PrinterIcon data-icon="inline-start" className="size-4 text-muted-foreground" />
                    <span className="min-w-0 flex-1 truncate text-left">{selectedPrinterLabel}</span>
                  </SelectTrigger>
                  <SelectContent align="end" className="min-w-56">
                    <SelectGroup>
                      {printerOptions.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </div>
            </div>
          </div>

          {statusTab === "needs_attention" && jobs.length > 0 ? (
            <div className="flex items-start gap-3 border-b border-amber-100 bg-amber-50/50 px-6 py-3">
              <AlertCircleIcon className="mt-0.5 size-5 shrink-0 text-amber-500" />
              <div className="min-w-0">
                <h4 className="text-sm font-medium text-amber-800">{t("history.attentionTitle")}</h4>
                <p className="mt-0.5 text-xs text-amber-600">
                  {t("history.attentionDescription")}
                </p>
              </div>
            </div>
          ) : null}

          <div className="hidden h-[min(560px,calc(100vh-22rem))] min-h-[320px] overflow-auto md:block">
            <Table>
              <TableHeader className="sticky top-0 z-10 bg-card">
                <TableRow>
                  <TableHead className="w-[34%] pl-5">{t("history.sourceId")}</TableHead>
                  <TableHead>{t("history.currentStatus")}</TableHead>
                  <TableHead>{t("history.targetPrinter")}</TableHead>
                  <TableHead>{t("history.updatedAt")}</TableHead>
                  <TableHead className="pr-5 text-right">{t("history.actions")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading && !jobs.length ? (
                  <TableSkeleton />
                ) : jobs.length ? (
                  jobs.map((job) => (
                    <TableRow key={job.job_id} className="align-top">
                      <TableCell className="pl-5">
                        <JobIdentity job={job} />
                      </TableCell>
                      <TableCell>
                        <JobStatusBlock job={job} />
                      </TableCell>
                      <TableCell>
                        <PrinterName job={job} />
                      </TableCell>
                      <TableCell>
                        <UpdatedAt value={job.updated_at} />
                      </TableCell>
                      <TableCell className="pr-5 text-right">
                        <JobActions
                          canceling={cancelingJobId === job.job_id}
                          job={job}
                          onCancel={() => void cancelJob(job)}
                        />
                      </TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={5}>
                      <EmptyState activeTabLabel={t(activeTab.labelKey)} search={normalizedSearch} />
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>

          <div className="h-[min(560px,calc(100vh-24rem))] min-h-[320px] divide-y overflow-y-auto md:hidden">
            {loading && !jobs.length ? (
              Array.from({ length: 4 }).map((_, index) => (
                <div key={index} className="space-y-3 px-4 py-4">
                  <div className="h-4 w-3/4 animate-pulse rounded bg-muted" />
                  <div className="h-3 w-1/2 animate-pulse rounded bg-muted" />
                  <div className="h-8 w-full animate-pulse rounded bg-muted" />
                </div>
              ))
            ) : jobs.length ? (
              jobs.map((job) => (
                <article key={job.job_id} className="space-y-3 px-4 py-4">
                  <div className="flex items-start justify-between gap-3">
                    <JobIdentity job={job} />
                    <JobActions
                      canceling={cancelingJobId === job.job_id}
                      job={job}
                      onCancel={() => void cancelJob(job)}
                    />
                  </div>
                  <JobStatusBlock job={job} />
                  <div className="grid gap-2 text-xs text-muted-foreground">
                    <PrinterName job={job} />
                    <UpdatedAt value={job.updated_at} />
                  </div>
                </article>
              ))
            ) : (
              <EmptyState activeTabLabel={t(activeTab.labelKey)} search={normalizedSearch} />
            )}
          </div>

          <div className="flex flex-col gap-3 border-t bg-muted/10 px-4 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-5">
            <div className="text-xs text-muted-foreground">
              {t("history.pageInfo", { page, totalPages, total })}
            </div>
            <div className="grid grid-cols-2 gap-2 sm:flex sm:items-center">
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={loading || page <= 1}
                onClick={() => setPage((current) => Math.max(1, current - 1))}
              >
                {t("history.previousPage")}
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={loading || page >= totalPages}
                onClick={() => setPage((current) => Math.min(totalPages, current + 1))}
              >
                {t("history.nextPage")}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function SectionHeader({
  activeJobsCount,
  needsAttentionCount,
}: {
  activeJobsCount: number | null;
  needsAttentionCount: number | null;
}) {
  const { t } = useI18n();

  return (
    <div className="overflow-hidden rounded-xl border bg-card">
      <div className="grid gap-4 p-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end sm:p-5">
        <div className="min-w-0">
          <div className="inline-flex items-center gap-2 rounded-full border bg-background px-2.5 py-1 text-xs text-muted-foreground">
            <PrinterIcon className="size-3.5" />
            DeepPrint Jobs
          </div>
          <h1 className="mt-3 font-heading text-2xl font-semibold tracking-tight text-foreground">
            {t("nav.history")}
          </h1>
          <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
            {t("history.summaryDescription")}
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 sm:w-64">
          <SummaryPill label={t("history.active")} value={activeJobsCount ?? "-"} tone="info" />
          <SummaryPill
            label={t("status.needs_attention")}
            value={needsAttentionCount ?? "-"}
            tone={(needsAttentionCount ?? 0) > 0 ? "danger" : "neutral"}
          />
        </div>
      </div>
    </div>
  );
}

function SummaryPill({
  label,
  tone,
  value,
}: {
  label: string;
  tone: "danger" | "info" | "neutral";
  value: number | string;
}) {
  return (
    <div
      className={cn(
        "rounded-lg border px-3 py-2",
        tone === "danger" && "border-destructive/20 bg-destructive/5",
        tone === "info" && "border-blue-500/20 bg-blue-500/5",
        tone === "neutral" && "bg-muted/30",
      )}
    >
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-0.5 text-lg font-semibold tabular-nums">{value}</div>
    </div>
  );
}

function JobIdentity({ job }: { job: JobResponse }) {
  return (
    <div className="flex min-w-0 items-start gap-3">
      <div
        className={cn(
          "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border",
          job.job_kind === "template"
            ? "border-violet-500/20 bg-violet-500/10 text-violet-700"
            : "border-blue-500/20 bg-blue-500/10 text-blue-700",
        )}
      >
        <FileTextIcon className="size-4" />
      </div>
      <div className="min-w-0">
        <div className="truncate font-medium text-foreground" title={summarizeJob(job)}>
          {summarizeJob(job)}
        </div>
        <div className="mt-0.5 truncate font-mono text-xs text-muted-foreground" title={job.job_id}>
          {formatJobId(job.job_id)}
        </div>
      </div>
    </div>
  );
}

function JobStatusBlock({ job }: { job: JobResponse }) {
  return (
    <div className="flex min-w-0 flex-col items-start gap-1.5">
      <StatusBadge status={job.status} />
      {job.last_error_message ? (
        <span className="max-w-80 truncate text-xs text-destructive" title={job.last_error_message}>
          {job.last_error_message}
        </span>
      ) : null}
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const iconClassName = "size-3";
  const normalized = status as JobStatus;
  if (normalized === "needs_attention") {
    return (
      <Badge className="border-amber-500/20 bg-amber-500/10 text-amber-800" variant="outline">
        <AlertCircleIcon className={iconClassName} />
        {statusLabel(status)}
      </Badge>
    );
  }
  if (normalized === "succeeded") {
    return (
      <Badge className="border-emerald-500/20 bg-emerald-500/10 text-emerald-700" variant="outline">
        <CheckCircle2Icon className={iconClassName} />
        {statusLabel(status)}
      </Badge>
    );
  }
  if (normalized === "failed") {
    return (
      <Badge variant="destructive">
        <XCircleIcon className={iconClassName} />
        {statusLabel(status)}
      </Badge>
    );
  }
  if (normalized === "canceled") {
    return (
      <Badge variant="outline">
        <MoreHorizontalIcon className={iconClassName} />
        {statusLabel(status)}
      </Badge>
    );
  }
  if (ACTIVE_JOB_STATUSES.has(normalized)) {
    return (
      <Badge className="border-blue-500/20 bg-blue-500/10 text-blue-700" variant="outline">
        <Loader2Icon
          className={cn(iconClassName, normalized === "printing" ? "animate-spin" : undefined)}
        />
        {statusLabel(status)}
      </Badge>
    );
  }
  return <Badge variant="outline">{statusLabel(status)}</Badge>;
}

function PrinterName({ job }: { job: JobResponse }) {
  const name = job.printer_name_snapshot ?? job.printer_id ?? "-";
  return (
    <div className="flex min-w-0 items-center gap-2 text-sm text-muted-foreground">
      <PrinterIcon className="size-3.5 shrink-0" />
      <span className="truncate" title={name}>
        {name}
      </span>
    </div>
  );
}

function UpdatedAt({ value }: { value: number }) {
  return (
    <div className="flex items-center gap-2 text-sm text-muted-foreground">
      <ClockIcon className="size-3.5 shrink-0" />
      <span>{formatUnixSec(value)}</span>
    </div>
  );
}

function JobActions({
  canceling,
  job,
  onCancel,
}: {
  canceling: boolean;
  job: JobResponse;
  onCancel: () => void;
}) {
  const { t } = useI18n();

  if (!canCancelJob(job.status)) {
    return <span className="text-xs text-muted-foreground">{t("history.noAction")}</span>;
  }
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      disabled={canceling}
      onClick={onCancel}
    >
      {canceling ? <Loader2Icon data-icon="inline-start" className="animate-spin" /> : null}
      {t("history.cancelJob")}
    </Button>
  );
}

function EmptyState({
  activeTabLabel,
  search,
}: {
  activeTabLabel: string;
  search: string | null;
}) {
  const { t } = useI18n();

  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center">
      <div className="flex size-14 items-center justify-center rounded-full bg-muted">
        <InboxIcon className="size-7 text-muted-foreground" />
      </div>
      <div className="mt-4 text-sm font-medium">{t("history.emptyTitle")}</div>
      <p className="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
        {search
          ? t("history.emptyFiltered", { search, tab: activeTabLabel })
          : t("history.emptyTab", { tab: activeTabLabel })}
      </p>
    </div>
  );
}

function TableSkeleton() {
  return (
    <>
      {Array.from({ length: 4 }).map((_, index) => (
        <TableRow key={index}>
          <TableCell className="pl-5">
            <div className="h-4 w-56 animate-pulse rounded bg-muted" />
            <div className="mt-2 h-3 w-40 animate-pulse rounded bg-muted" />
          </TableCell>
          <TableCell>
            <div className="h-5 w-20 animate-pulse rounded-full bg-muted" />
          </TableCell>
          <TableCell>
            <div className="h-4 w-36 animate-pulse rounded bg-muted" />
          </TableCell>
          <TableCell>
            <div className="h-4 w-32 animate-pulse rounded bg-muted" />
          </TableCell>
          <TableCell className="pr-5">
            <div className="ml-auto h-7 w-20 animate-pulse rounded bg-muted" />
          </TableCell>
        </TableRow>
      ))}
    </>
  );
}

function getStatusParam(tab: JobStatusTab) {
  const config = STATUS_TABS.find((item) => item.value === tab) ?? STATUS_TABS[0];
  return config.statuses.join(",");
}

function summarizeJob(job: JobResponse) {
  if (job.source_file_name) return job.source_file_name;
  if (job.job_kind === "template") return translate(getCurrentLocale(), "print.templatePrintJob");
  return job.job_kind || translate(getCurrentLocale(), "print.printJob");
}

function formatJobId(jobId: string) {
  if (jobId.length <= 18) return jobId;
  return `${jobId.slice(0, 10)}...${jobId.slice(-6)}`;
}

function canCancelJob(status: string) {
  return status === "queued" || status === "printing" || status === "needs_attention";
}
