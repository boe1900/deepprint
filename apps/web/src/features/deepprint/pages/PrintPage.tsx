import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowRightIcon,
  CheckCircle2Icon,
  ChevronDownIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  FileImageIcon,
  FileTextIcon,
  Loader2Icon,
  PlusIcon,
  PrinterIcon,
  UploadIcon,
  XIcon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { AppPage } from "@/App";
import type { DeepprintController } from "../controller";
import { createPrinterDetailQueryOptions, createRecentJobsQueryOptions } from "../queries";
import type { JobResponse, PrinterCapabilities, PrinterInfo } from "../types";
import { buildObjectUrl, formatBytes, formatUnixSec, statusLabel } from "../utils";
import { printerStateLabel, selectDefaultPrinter, statusBadgeVariant } from "../ui";

type PrintPageProps = {
  controller: DeepprintController;
  templatePrintRevision: number;
  onNavigate: (page: AppPage) => void;
};

type UploadedPrintFile = {
  id: string;
  name: string;
  size: number;
  type: "pdf" | "image" | "template";
  pages: number;
  file: File | null;
  localPreviewUrl: string | null;
};

type ColorMode = "color" | "monochrome";
type DuplexMode = "one-sided" | "two-sided-long-edge" | "two-sided-short-edge";
type OrientationMode = "portrait" | "landscape";
type ScalingMode = "auto" | "auto-fit" | "fit" | "fill" | "none";
type PaperSize = string;
type PaperType = string;
type PrintCenterTab = "settings" | "tasks";
type SelectOption = { value: string; label: string };

const PRINTER_DEFAULT_VALUE = "__printer_default__";

export function PrintPage({
  controller,
  onNavigate,
  templatePrintRevision,
}: PrintPageProps) {
  const { actions, agent, jobs, ui, writes } = controller;
  const queryClient = useQueryClient();
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const printFilesRef = useRef<UploadedPrintFile[]>([]);
  const [printFiles, setPrintFiles] = useState<UploadedPrintFile[]>([]);
  const [activeFileId, setActiveFileId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<PrintCenterTab>("settings");
  const [previewPage, setPreviewPage] = useState(1);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [batchPrinting, setBatchPrinting] = useState(false);
  const [colorMode, setColorMode] = useState<ColorMode>("color");
  const [duplexMode, setDuplexMode] = useState<DuplexMode>("one-sided");
  const [paperSize, setPaperSize] = useState<PaperSize>("iso_a4_210x297mm");
  const [paperType, setPaperType] = useState<PaperType>(PRINTER_DEFAULT_VALUE);
  const [orientation, setOrientation] = useState<OrientationMode>("portrait");
  const [scaleMode, setScaleMode] = useState<ScalingMode>("fit");
  const [pageRange, setPageRange] = useState("");
  const [appliedTemplatePrintRevision, setAppliedTemplatePrintRevision] = useState(0);

  const selectedPrinterId = writes.directPrinterId || writes.createPrinterId;
  const selectedPrinter = useMemo(
    () => agent.printers.find((printer) => printer.id === selectedPrinterId) ?? null,
    [agent.printers, selectedPrinterId],
  );
  const selectedPrinterDetailQuery = useQuery({
    ...createPrinterDetailQueryOptions({
      baseUrl: ui.baseUrl,
      printerId: selectedPrinterId,
      timeoutMs: ui.requestTimeouts.printers,
    }),
    enabled: Boolean(agent.health && selectedPrinterId),
  });
  const selectedPrinterDetail = selectedPrinterDetailQuery.data ?? null;
  const capabilities = selectedPrinterDetail?.capabilities ?? null;
  const capabilityOptions = useMemo(
    () => buildCapabilityOptions(capabilities),
    [capabilities],
  );
  const supportsPageRanges = capabilities?.supports_page_ranges === true;
  const copiesCapability = capabilities?.copies ?? null;
  const minCopies = Math.max(1, copiesCapability?.min ?? 1);
  const maxCopies = Math.max(minCopies, Math.min(100, copiesCapability?.max ?? 100));
  const capabilityLoading = Boolean(selectedPrinterId && selectedPrinterDetailQuery.isFetching && !selectedPrinterDetail);
  const capabilityError = selectedPrinterDetailQuery.error instanceof Error
    ? selectedPrinterDetailQuery.error.message
    : selectedPrinterDetailQuery.isError
      ? "无法读取当前打印机能力"
      : null;
  const activeFile = useMemo(
    () => printFiles.find((file) => file.id === activeFileId) ?? printFiles[0] ?? null,
    [activeFileId, printFiles],
  );

  useEffect(() => {
    if (!agent.printers.length) return;
    const fallback = selectDefaultPrinter(agent.printers);
    if (!writes.createPrinterId) writes.setCreatePrinterId(fallback);
    if (!writes.directPrinterId) writes.setDirectPrinterId(fallback);
  }, [
    agent.printers,
    writes.createPrinterId,
    writes.directPrinterId,
    writes.setCreatePrinterId,
    writes.setDirectPrinterId,
  ]);

  const choosePrinter = (printerId: string) => {
    writes.setCreatePrinterId(printerId);
    writes.setDirectPrinterId(printerId);
  };

  const refreshPrintersAndCapabilities = async () => {
    await actions.refreshAll(false);
    if (selectedPrinterId) {
      await queryClient.invalidateQueries({
        queryKey: createPrinterDetailQueryOptions({
          baseUrl: ui.baseUrl,
          printerId: selectedPrinterId,
          timeoutMs: ui.requestTimeouts.printers,
        }).queryKey,
      });
    }
  };

  useEffect(() => {
    if (!capabilities) return;
    setColorMode((current) => firstSupportedValue(capabilityOptions.color, current, "color") as ColorMode);
    setDuplexMode((current) => {
      const next = firstSupportedValue(capabilityOptions.sides, current, "one-sided") as DuplexMode;
      if (next !== current) writes.setCreateDuplex(next);
      return next;
    });
    setPaperSize((current) => {
      const next = firstSupportedValue(
        capabilityOptions.media,
        current,
        capabilities.media_default ?? "iso_a4_210x297mm",
      );
      if (next !== current) writes.setCreatePaperSize(next);
      return next;
    });
    setPaperType((current) => firstSupportedValue(capabilityOptions.mediaType, current, PRINTER_DEFAULT_VALUE));
    setOrientation((current) => firstSupportedValue(capabilityOptions.orientation, current, "portrait") as OrientationMode);
    setScaleMode((current) => firstSupportedValue(capabilityOptions.scaling, current, "fit") as ScalingMode);
    if (copiesCapability) {
      const parsed = Number.parseInt(writes.createCopies.trim(), 10);
      const normalized = Number.isFinite(parsed)
        ? Math.min(maxCopies, Math.max(minCopies, parsed))
        : copiesCapability.default ?? minCopies;
      if (String(normalized) !== writes.createCopies) {
        writes.setCreateCopies(String(normalized));
      }
    }
    if (!supportsPageRanges) setPageRange("");
  }, [
    capabilities,
    capabilityOptions.color,
    capabilityOptions.media,
    capabilityOptions.mediaType,
    capabilityOptions.orientation,
    capabilityOptions.scaling,
    capabilityOptions.sides,
    supportsPageRanges,
    copiesCapability,
    maxCopies,
    minCopies,
    writes.createCopies,
    writes.setCreateCopies,
    writes.setCreateDuplex,
    writes.setCreatePaperSize,
  ]);

  const recentJobsQuery = useQuery({
    ...createRecentJobsQueryOptions({
      baseUrl: ui.baseUrl,
      limit: 5,
      printerId: selectedPrinterId || null,
      timeoutMs: ui.requestTimeouts.jobStatus,
    }),
    enabled: Boolean(agent.health),
  });
  const recentJobs = recentJobsQuery.data?.jobs ?? [];
  const recentLoading = recentJobsQuery.isFetching;

  const refreshRecentJobs = async () => {
    await queryClient.invalidateQueries({
      queryKey: createRecentJobsQueryOptions({
        baseUrl: ui.baseUrl,
        limit: 5,
        printerId: selectedPrinterId || null,
        timeoutMs: ui.requestTimeouts.jobStatus,
      }).queryKey,
    });
  };

  const currentPrintOptions = useMemo(() => {
    return buildCurrentPrintOptions({
      colorMode,
      duplexMode,
      orientation,
      pageRange,
      paperSize,
      paperType,
      scaleMode,
      capabilityOptions,
      copiesSupported: Boolean(copiesCapability),
      supportsPageRanges,
      copies: writes.createCopies,
    });
  }, [
    colorMode,
    duplexMode,
    orientation,
    pageRange,
    paperSize,
    paperType,
    scaleMode,
    capabilityOptions,
    copiesCapability,
    supportsPageRanges,
    writes.createCopies,
  ]);

  const syncPrintFiles = (updater: (current: UploadedPrintFile[]) => UploadedPrintFile[]) => {
    const next = updater(printFilesRef.current);
    printFilesRef.current = next;
    setPrintFiles(next);
  };

  useEffect(() => {
    if (
      templatePrintRevision <= 0 ||
      templatePrintRevision === appliedTemplatePrintRevision ||
      !writes.createTemplateContent.trim()
    ) {
      return;
    }
    setAppliedTemplatePrintRevision(templatePrintRevision);
    const templateFile = buildTemplateUploadedFile(writes.createTemplateContent, writes.createDataJson);
    syncPrintFiles((current) => {
      current
        .filter((file) => file.type === "template" && file.localPreviewUrl)
        .forEach((file) => URL.revokeObjectURL(file.localPreviewUrl as string));
      return [templateFile, ...current.filter((file) => file.type !== "template")];
    });
    setActiveFileId(templateFile.id);
    setPreviewPage(1);
    void actions.onPreviewTypst({ printOptions: currentPrintOptions });
  }, [
    actions,
    appliedTemplatePrintRevision,
    currentPrintOptions,
    templatePrintRevision,
    writes.createDataJson,
    writes.createTemplateContent,
  ]);

  useEffect(() => {
    return () => {
      printFilesRef.current.forEach((file) => {
        if (file.localPreviewUrl) URL.revokeObjectURL(file.localPreviewUrl);
      });
    };
  }, []);

  const advancedSummary = useMemo(() => {
    const paperSizeLabel = formatPaperLabel(paperSize, capabilityOptions.media);
    const paperTypeLabel = capabilityOptions.mediaType.find((option) => option.value === paperType)?.label ?? formatKeywordLabel(paperType);
    const scaleLabel = capabilityOptions.scaling.find((option) => option.value === scaleMode)?.label ?? formatKeywordLabel(scaleMode);
    return `${paperSizeLabel} / ${paperTypeLabel} / ${scaleLabel}`;
  }, [capabilityOptions.media, capabilityOptions.mediaType, capabilityOptions.scaling, paperSize, paperType, scaleMode]);

  const printableFiles = useMemo(
    () => printFiles.filter((file) => isPrintableFile(file, writes.createTemplateContent)),
    [printFiles, writes.createTemplateContent],
  );
  const canPrint = Boolean(
    selectedPrinter &&
      selectedPrinter.enabled &&
      capabilities &&
      !capabilityLoading &&
      printableFiles.length > 0,
  );

  const selectActiveFile = (fileId: string | null) => {
    const nextFile = printFilesRef.current.find((file) => file.id === fileId) ?? null;
    setActiveFileId(nextFile?.id ?? null);
    setPreviewPage(1);
    if (nextFile?.file) {
      writes.setDirectSelectedFile(nextFile.file);
    } else {
      writes.setDirectSelectedFile(null);
    }
    if (nextFile?.type === "template") {
      void actions.onPreviewTypst({ printOptions: currentPrintOptions });
    }
  };

  const addSelectedFiles = (selectedFiles: File[]) => {
    if (!selectedFiles.length) return;

    const nextFiles = selectedFiles
      .map((file, index) => buildUploadedFile(file, index))
      .filter((file): file is UploadedPrintFile => Boolean(file));

    if (!nextFiles.length) {
      ui.setNotice({ kind: "error", message: "当前仅支持选择 PDF 或图片文件" });
      event.target.value = "";
      return;
    }

    if (nextFiles.length !== selectedFiles.length) {
      ui.setNotice({ kind: "error", message: "已跳过不支持的文件，仅添加 PDF 或图片" });
    }

    syncPrintFiles((current) => [...current, ...nextFiles]);
    if (!activeFile) {
      selectActiveFile(nextFiles[0].id);
    }
  };

  const handleFileSelected = (event: React.ChangeEvent<HTMLInputElement>) => {
    addSelectedFiles(Array.from(event.target.files ?? []));
    event.target.value = "";
  };

  const removePrintFile = (fileId: string) => {
    const currentFiles = printFilesRef.current;
    const removedFile = currentFiles.find((file) => file.id === fileId);
    const nextFiles = currentFiles.filter((file) => file.id !== fileId);
    if (removedFile?.localPreviewUrl) {
      URL.revokeObjectURL(removedFile.localPreviewUrl);
    }
    syncPrintFiles(() => nextFiles);
    if (activeFileId === fileId) {
      selectActiveFile(nextFiles[0]?.id ?? null);
    }
  };

  const clearPrintFiles = () => {
    printFilesRef.current.forEach((file) => {
      if (file.localPreviewUrl) URL.revokeObjectURL(file.localPreviewUrl);
    });
    syncPrintFiles(() => []);
    selectActiveFile(null);
  };

  const handlePrint = async () => {
    if (!canPrint) return;
    setBatchPrinting(true);
    try {
      for (const file of printableFiles) {
        if (file.type === "template") {
          await actions.onCreateJob(createSyntheticSubmitEvent(), {
            printOptions: currentPrintOptions,
          });
          continue;
        }
        if (file.file) {
          await actions.onCreateDirectJob(createSyntheticSubmitEvent(), {
            file: file.file,
            printOptions: currentPrintOptions,
          });
        }
      }
      setActiveTab("tasks");
      await refreshRecentJobs();
    } finally {
      setBatchPrinting(false);
    }
  };

  return (
    <main className="flex min-h-0 flex-1 flex-col bg-[radial-gradient(circle_at_top_left,color-mix(in_srgb,var(--muted)_80%,transparent)_0,transparent_34rem),var(--background)]">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        accept="application/pdf,image/*,.pdf,.png,.jpg,.jpeg,.gif,.webp,.bmp"
        className="hidden"
        onChange={handleFileSelected}
      />
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto lg:flex-row lg:overflow-hidden">
        <section className="flex min-h-[58dvh] min-w-0 flex-col gap-3 p-3 sm:p-4 lg:min-h-0 lg:flex-1 lg:p-6">
          {activeFile ? (
            <>
              <PreviewPanel
                file={activeFile}
                page={previewPage}
                onPageChange={setPreviewPage}
                pdfUrl={writes.previewPdfUrl}
                previewLoading={writes.previewLoading}
                paperSize={paperSize}
                orientation={orientation}
              />
              <FileStrip
                activeFileId={activeFile.id}
                files={printFiles}
                onAdd={() => fileInputRef.current?.click()}
                onClearAll={clearPrintFiles}
                onRemove={removePrintFile}
                onSelect={selectActiveFile}
              />
            </>
          ) : (
            <UploadDropzone
              onDropFiles={addSelectedFiles}
              onSelect={() => fileInputRef.current?.click()}
            />
          )}
        </section>

        <aside className="flex w-full shrink-0 flex-col border-t bg-card/95 shadow-[0_-12px_40px_rgba(15,23,42,0.04)] backdrop-blur lg:h-full lg:w-[390px] lg:border-l lg:border-t-0">
          <div className="sticky top-0 z-20 flex h-14 shrink-0 items-center gap-6 border-b bg-card/95 px-4 backdrop-blur lg:h-16 lg:px-6">
            <TabButton active={activeTab === "settings"} onClick={() => setActiveTab("settings")}>
              打印设置
            </TabButton>
            <TabButton active={activeTab === "tasks"} onClick={() => setActiveTab("tasks")}>
              任务队列
              {recentJobs.length ? (
                <span className="ml-2 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-muted px-1.5 text-[11px] text-muted-foreground">
                  {recentJobs.length}
                </span>
              ) : null}
            </TabButton>
          </div>

          {activeTab === "settings" ? (
            <>
              <div className="flex-1 space-y-5 px-4 py-4 lg:overflow-y-auto lg:px-6 lg:py-5">
                {selectedPrinter && capabilityLoading ? (
                  <div className="rounded-2xl border bg-muted/40 px-4 py-3 text-sm text-muted-foreground">
                    正在读取当前打印机能力...
                  </div>
                ) : null}
                {selectedPrinter && capabilityError ? (
                  <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                    无法读取当前打印机能力：{capabilityError}
                  </div>
                ) : null}

                <PrinterPicker
                  disabled={agent.printers.length === 0 || agent.refreshingAll}
                  onChange={choosePrinter}
                  onManage={() => onNavigate("printers")}
                  onRefresh={() => void refreshPrintersAndCapabilities()}
                  printers={agent.printers}
                  refreshing={agent.refreshingAll}
                  selectedPrinter={selectedPrinter}
                  value={selectedPrinterId}
                />

                <div className="h-px bg-border/70" />

                <div className="space-y-4">
                  {copiesCapability ? (
                    <CopiesControl
                      max={maxCopies}
                      min={minCopies}
                      onChange={writes.setCreateCopies}
                      value={writes.createCopies}
                    />
                  ) : null}
                  {capabilityOptions.color.length ? (
                    <div className="space-y-2">
                      <div className="text-sm font-medium">颜色模式</div>
                      <Segmented
                        value={colorMode}
                        options={capabilityOptions.color}
                        onChange={(value) => setColorMode(value as ColorMode)}
                      />
                    </div>
                  ) : null}
                  {capabilityOptions.sides.length ? (
                    <LabeledSelect
                      label="单双面"
                      value={duplexMode}
                      onChange={(value) => {
                        setDuplexMode(value as DuplexMode);
                        writes.setCreateDuplex(value);
                      }}
                      options={capabilityOptions.sides}
                    />
                  ) : null}
                </div>

                <div className="rounded-2xl border bg-background/70">
                  <button
                    type="button"
                    onClick={() => setAdvancedOpen((current) => !current)}
                    className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
                  >
                    <div className="min-w-0">
                      <div className="text-sm font-medium">更多高级选项</div>
                      <div className="mt-1 truncate text-xs text-muted-foreground">{advancedSummary}</div>
                    </div>
                    <ChevronDownIcon className={`size-4 shrink-0 transition-transform ${advancedOpen ? "rotate-180" : ""}`} />
                  </button>

                  {advancedOpen ? (
                    <div className="space-y-4 border-t px-4 py-4">
                      {capabilityOptions.media.length ? (
                        <LabeledSelect
                          label="纸张大小"
                          value={paperSize}
                          onChange={(value) => {
                            setPaperSize(value);
                            writes.setCreatePaperSize(value);
                          }}
                          options={capabilityOptions.media}
                        />
                      ) : null}
                      {capabilityOptions.mediaType.length ? (
                        <LabeledSelect label="纸张类型" value={paperType} onChange={setPaperType} options={capabilityOptions.mediaType} />
                      ) : null}
                      {capabilityOptions.orientation.length ? (
                        <div className="space-y-2">
                          <div className="text-xs font-medium text-muted-foreground">页面方向</div>
                          <Segmented
                            value={orientation}
                            options={capabilityOptions.orientation}
                            onChange={(value) => setOrientation(value as OrientationMode)}
                          />
                        </div>
                      ) : null}
                      {capabilityOptions.scaling.length ? (
                        <LabeledSelect
                          label="缩放模式"
                          value={scaleMode}
                          onChange={(value) => setScaleMode(value as ScalingMode)}
                          options={capabilityOptions.scaling}
                        />
                      ) : null}
                      {supportsPageRanges ? (
                        <div className="space-y-2">
                          <div className="text-xs font-medium text-muted-foreground">页码范围</div>
                          <input
                            value={pageRange}
                            onChange={(event) => setPageRange(event.target.value)}
                            placeholder="例如 1-5 8"
                            className="h-10 w-full rounded-xl border bg-card px-3 text-sm outline-none transition-colors focus:border-ring focus:ring-3 focus:ring-ring/20"
                          />
                        </div>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              </div>

              <div className="sticky bottom-0 z-20 shrink-0 border-t bg-card/95 p-3 backdrop-blur lg:p-5">
                <Button
                  className="h-11 w-full rounded-2xl text-sm shadow-[0_10px_24px_rgba(15,23,42,0.12)] lg:h-12 lg:text-base"
                  disabled={!canPrint || writes.createLoading || writes.directLoading || batchPrinting}
                  onClick={() => void handlePrint()}
                >
                  {batchPrinting || writes.createLoading || writes.directLoading ? (
                    <span className="inline-flex items-center gap-2">
                      <Loader2Icon className="size-4 animate-spin" />
                      正在提交打印机...
                    </span>
                  ) : (
                    `打印 ${printableFiles.length ? `${printableFiles.length} 个文件` : ""}`
                  )}
                </Button>
              </div>
            </>
          ) : (
            <TaskQueuePanel
              currentJob={jobs.jobResult}
              jobs={recentJobs}
              loading={recentLoading}
              onOpenHistory={() => onNavigate("history")}
            />
          )}
        </aside>
      </div>
    </main>
  );
}

function UploadDropzone({
  onDropFiles,
  onSelect,
}: {
  onDropFiles: (files: File[]) => void;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      onDragOver={(event) => {
        event.preventDefault();
      }}
      onDrop={(event) => {
        event.preventDefault();
        onDropFiles(Array.from(event.dataTransfer.files));
      }}
      className="group flex min-h-[52dvh] flex-1 flex-col items-center justify-center rounded-3xl border border-dashed bg-card/70 p-6 text-center shadow-sm transition-colors hover:border-foreground/20 hover:bg-card lg:min-h-0"
    >
      <div className="flex size-16 items-center justify-center rounded-3xl bg-primary/10 text-primary transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
        <UploadIcon className="size-7" />
      </div>
      <div className="mt-5 text-base font-medium">选择或拖拽文件到此处</div>
      <div className="mt-2 max-w-[360px] text-sm leading-6 text-muted-foreground">
        支持批量选择 PDF 和图片文件。图片会由后端转换为 PDF，再进入真实打印链路。
      </div>
      <div className="mt-5 rounded-full border bg-background px-4 py-2 text-sm text-foreground shadow-sm">
        选择本地文件
      </div>
    </button>
  );
}

function FileStrip({
  activeFileId,
  files,
  onAdd,
  onClearAll,
  onRemove,
  onSelect,
}: {
  activeFileId: string | null;
  files: UploadedPrintFile[];
  onAdd: () => void;
  onClearAll: () => void;
  onRemove: (fileId: string) => void;
  onSelect: (fileId: string) => void;
}) {
  return (
    <div className="shrink-0 space-y-2">
      <div className="flex items-center justify-between gap-3 px-1">
        <div className="text-xs font-medium text-muted-foreground">
          待打印文件 · {files.length}
        </div>
        {files.length ? (
          <button
            type="button"
            onClick={onClearAll}
            className="text-xs text-muted-foreground transition-colors hover:text-destructive"
          >
            清空
          </button>
        ) : null}
      </div>
      <div className="flex gap-2 overflow-x-auto pb-1 lg:gap-3">
        <button
          type="button"
          onClick={onAdd}
          className="flex h-16 w-16 shrink-0 flex-col items-center justify-center gap-1 rounded-2xl border border-dashed bg-card/60 text-muted-foreground transition-colors hover:bg-card hover:text-foreground sm:h-24 sm:w-24"
        >
          <PlusIcon className="size-5" />
          <span className="hidden text-[11px] font-medium sm:block">继续添加</span>
        </button>
        {files.map((file) => {
          const active = file.id === activeFileId;
          return (
            <div
              key={file.id}
              role="button"
              tabIndex={0}
              onClick={() => onSelect(file.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelect(file.id);
                }
              }}
              className={[
                "group relative flex h-16 w-36 shrink-0 flex-col justify-between rounded-2xl border bg-card p-2 text-left shadow-sm transition-all sm:h-24 sm:w-44 sm:p-3",
                active ? "border-primary ring-2 ring-primary/15" : "hover:border-foreground/20",
              ].join(" ")}
            >
              <div className="flex items-start justify-between gap-2">
                <div className={`rounded-xl p-1.5 ${file.type === "image" ? "bg-blue-500/10 text-blue-600" : "bg-red-500/10 text-red-600"}`}>
                  {file.type === "image" ? <FileImageIcon className="size-4" /> : <FileTextIcon className="size-4" />}
                </div>
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    onRemove(file.id);
                  }}
                  className="flex size-6 items-center justify-center rounded-lg text-muted-foreground opacity-70 transition-colors hover:bg-destructive/10 hover:text-destructive sm:opacity-0 sm:group-hover:opacity-100"
                >
                  <XIcon className="size-3.5" />
                </button>
              </div>
              <div className="min-w-0">
                <div className="truncate text-xs font-medium sm:text-sm" title={file.name}>
                  {file.name}
                </div>
                <div className="mt-0.5 hidden text-[11px] text-muted-foreground sm:block">
                  {formatBytes(file.size)} · {file.pages} 页
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function PreviewPanel({
  file,
  onPageChange,
  page,
  pdfUrl,
  previewLoading,
  paperSize,
  orientation,
}: {
  file: UploadedPrintFile;
  onPageChange: (value: number | ((current: number) => number)) => void;
  page: number;
  pdfUrl: string | null;
  previewLoading: boolean;
  paperSize: PaperSize;
  orientation: OrientationMode;
}) {
  const paper = getPaperDimensionsMm(paperSize, orientation);
  const paperAspect = paper.width / paper.height;

  return (
    <div className="flex min-h-[46dvh] flex-1 flex-col overflow-hidden rounded-3xl border bg-card shadow-sm lg:min-h-0">
      <div className="mb-4 flex items-center justify-between">
        <div className="min-w-0 px-5 pt-5">
          <div className="truncate text-sm font-medium">{file.name}</div>
          <div className="mt-1 text-xs text-muted-foreground">
            {file.type === "template"
              ? "模板 PDF 预览"
              : file.type === "pdf"
                ? "PDF 本地文件预览"
                : "图片本地文件预览"}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2 px-5 pt-5 text-xs text-muted-foreground">
          <button
            type="button"
            className="flex size-8 items-center justify-center rounded-lg border bg-background disabled:opacity-40"
            disabled={page <= 1}
            onClick={() => onPageChange((current) => Math.max(1, current - 1))}
          >
            <ChevronLeftIcon className="size-4" />
          </button>
          <span>
            {page} / {file.pages}
          </span>
          <button
            type="button"
            className="flex size-8 items-center justify-center rounded-lg border bg-background disabled:opacity-40"
            disabled={page >= file.pages}
            onClick={() => onPageChange((current) => Math.min(file.pages, current + 1))}
          >
            <ChevronRightIcon className="size-4" />
          </button>
        </div>
      </div>

      {file.type === "template" && pdfUrl ? (
        <iframe title="Template Preview" src={pdfUrl} className="mx-5 mb-5 min-h-[420px] flex-1 rounded-[24px] border bg-background lg:min-h-0" />
      ) : file.type === "pdf" && file.localPreviewUrl ? (
        <iframe title="PDF File Preview" src={file.localPreviewUrl} className="mx-5 mb-5 min-h-[420px] flex-1 rounded-[24px] border bg-background lg:min-h-0" />
      ) : file.type === "image" && file.localPreviewUrl ? (
        <div className="mx-5 mb-5 flex min-h-[420px] flex-1 items-center justify-center overflow-hidden rounded-[24px] border bg-gradient-to-b from-background to-muted p-5 lg:min-h-0">
          <div
            className="relative flex max-h-full max-w-full items-center justify-center overflow-hidden rounded-xl border bg-white p-[7%] shadow-[0_20px_60px_rgba(0,0,0,0.16)]"
            style={{ aspectRatio: `${paperAspect}` }}
          >
            <img
              src={file.localPreviewUrl}
              alt={file.name}
              className="h-full w-full object-contain"
            />
            <div className="pointer-events-none absolute left-3 top-3 rounded-full border bg-white/90 px-2 py-1 text-[10px] font-medium text-neutral-500 shadow-sm">
              {formatPaperLabel(paperSize)} · {orientation === "landscape" ? "横向" : "纵向"}
            </div>
          </div>
        </div>
      ) : (
        <div className="mx-5 mb-5 flex min-h-[420px] flex-1 items-center rounded-[28px] border bg-gradient-to-b from-background to-muted p-4 lg:min-h-0">
          <div className="mx-auto flex aspect-[0.78] max-h-[520px] min-h-[420px] w-full max-w-[420px] flex-col rounded-[24px] border bg-background px-6 py-6 shadow-[0_20px_60px_rgba(0,0,0,0.12)]">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              {file.type === "image" ? <FileImageIcon className="size-4" /> : <FileTextIcon className="size-4" />}
              {previewLoading ? "正在生成预览..." : file.name}
            </div>
            <div className="mt-6 rounded-2xl border bg-card px-4 py-4 text-sm text-muted-foreground">
              当前文件暂无可显示预览。
            </div>
            <div className="mt-auto rounded-2xl border bg-card px-4 py-4">
              <div className="text-sm font-medium">{file.name}</div>
              <div className="mt-1 text-xs text-muted-foreground">
                {file.type === "image" ? "图片文件" : `${file.pages} 页 PDF`}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function TaskQueuePanel({
  currentJob,
  jobs,
  loading,
  onOpenHistory,
}: {
  currentJob: JobResponse | null;
  jobs: JobResponse[];
  loading: boolean;
  onOpenHistory: () => void;
}) {
  const displayJobs = currentJob
    ? [currentJob, ...jobs.filter((job) => job.job_id !== currentJob.job_id)].slice(0, 5)
    : jobs.slice(0, 5);

  return (
    <div className="min-h-[42dvh] flex-1 space-y-3 bg-muted/20 p-4 lg:min-h-0 lg:overflow-y-auto lg:p-5">
      <div className="flex items-center justify-between px-1">
        <div>
          <div className="text-sm font-medium">最近任务</div>
          <div className="mt-1 text-xs text-muted-foreground">
            {loading ? "正在刷新..." : "最近 5 条真实打印任务"}
          </div>
        </div>
        <button
          type="button"
          className="inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          onClick={onOpenHistory}
        >
          查看全部
          <ArrowRightIcon className="size-3" />
        </button>
      </div>

      {displayJobs.length === 0 ? (
        <div className="flex min-h-[260px] flex-col items-center justify-center rounded-3xl border bg-card px-6 py-10 text-center text-sm text-muted-foreground">
          <PrinterIcon className="mb-3 size-10 opacity-25" />
          <div className="font-medium text-foreground">暂无打印任务</div>
          <div className="mt-1 text-xs">
            暂无最近打印任务
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {displayJobs.map((job) => {
            const processing = isProcessingStatus(job.status);
            return (
              <div
                key={job.job_id}
                className={[
                  "relative overflow-hidden rounded-2xl border bg-card px-4 py-3 shadow-sm",
                  processing ? "border-primary/25" : "",
                ].join(" ")}
              >
                {processing ? (
                  <div className="absolute left-0 top-0 h-1 w-full overflow-hidden bg-primary/10">
                    <div className="h-full w-1/2 animate-pulse rounded-full bg-primary" />
                  </div>
                ) : null}
                <div className="flex items-start justify-between gap-3 pt-1">
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">{summarizeJob(job)}</div>
                    <div className="mt-1 flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted-foreground">
                      <span>{formatUnixSec(job.updated_at)}</span>
                      {job.printer_name_snapshot ? (
                        <>
                          <span className="text-border">·</span>
                          <span className="max-w-[160px] truncate">{job.printer_name_snapshot}</span>
                        </>
                      ) : null}
                    </div>
                  </div>
                  <Badge variant={statusBadgeVariant(job.status)} className="shrink-0">
                    {processing ? <Loader2Icon className="size-3 animate-spin" /> : isSucceededStatus(job.status) ? <CheckCircle2Icon className="size-3" /> : null}
                    {statusLabel(job.status)}
                  </Badge>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function PrinterPicker({
  disabled,
  onChange,
  onManage,
  onRefresh,
  printers,
  refreshing,
  selectedPrinter,
  value,
}: {
  disabled: boolean;
  onChange: (printerId: string) => void;
  onManage: () => void;
  onRefresh: () => void;
  printers: PrinterInfo[];
  refreshing: boolean;
  selectedPrinter: PrinterInfo | null;
  value: string;
}) {
  return (
    <div className="space-y-3">
      <div className="space-y-2">
        <div className="text-sm font-medium">目标打印机</div>
        <NativeSelect
          value={value}
          onChange={onChange}
          disabled={disabled}
          options={
            printers.length
              ? printers.map((printerItem) => ({ value: printerItem.id, label: printerItem.name }))
              : [{ value: "", label: "请先导入或添加打印机" }]
          }
        />
      </div>
      {selectedPrinter ? (
        <div className="rounded-2xl border bg-background/70 px-4 py-4">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="truncate text-sm font-medium">{selectedPrinter.name}</div>
              <div className="mt-1 truncate text-xs text-muted-foreground">{selectedPrinter.uri}</div>
            </div>
            <Badge variant={statusBadgeVariant(selectedPrinter.state)}>
              {printerStateLabel(selectedPrinter)}
            </Badge>
          </div>
          <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span>{selectedPrinter.is_default ? "默认打印机" : "非默认"}</span>
            <span>{selectedPrinter.enabled ? "已启用" : "已停用"}</span>
            <span>最近校验：{formatUnixSec(selectedPrinter.last_validated_at)}</span>
          </div>
        </div>
      ) : (
        <div className="rounded-2xl border bg-background/70 px-4 py-5 text-center text-sm text-muted-foreground">
          还没有可用打印机，请先在打印机中添加。
        </div>
      )}
      <div className="grid grid-cols-2 gap-2">
        <Button type="button" variant="outline" disabled={refreshing} onClick={onRefresh}>
          {refreshing ? "刷新中..." : "刷新"}
        </Button>
        <Button type="button" variant="outline" onClick={onManage}>
          管理打印机
        </Button>
      </div>
    </div>
  );
}

function TabButton({
  active,
  children,
  onClick,
}: {
  active: boolean;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={[
        "relative flex h-full items-center text-sm font-semibold transition-colors",
        active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
      ].join(" ")}
    >
      {children}
      {active ? <span className="absolute inset-x-0 bottom-0 h-0.5 rounded-full bg-primary" /> : null}
    </button>
  );
}

function CopiesControl({
  max,
  min,
  onChange,
  value,
}: {
  max: number;
  min: number;
  onChange: (value: string) => void;
  value: string;
}) {
  const current = Number.parseInt(value, 10);
  const normalized = Number.isFinite(current) ? Math.min(max, Math.max(min, current)) : min;
  const step = (delta: number) => {
    onChange(String(Math.min(max, Math.max(min, normalized + delta))));
  };

  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <div className="text-sm font-medium">打印份数</div>
        <div className="mt-1 text-xs text-muted-foreground">
          {min}-{max} 份
        </div>
      </div>
      <div className="flex h-10 w-32 items-center overflow-hidden rounded-xl border bg-background">
        <button
          type="button"
          onClick={() => step(-1)}
          disabled={normalized <= min}
          className="h-full px-3 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:opacity-40"
        >
          -
        </button>
        <input
          type="number"
          min={min}
          max={max}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="min-w-0 flex-1 bg-transparent text-center text-sm font-medium outline-none"
        />
        <button
          type="button"
          onClick={() => step(1)}
          disabled={normalized >= max}
          className="h-full px-3 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground disabled:opacity-40"
        >
          +
        </button>
      </div>
    </div>
  );
}

function Segmented({
  onChange,
  options,
  value,
}: {
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
  value: string;
}) {
  return (
    <div className="flex rounded-xl border bg-background p-1">
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            className={[
              "flex-1 rounded-lg px-3 py-2 text-sm transition-colors",
              active ? "bg-muted text-foreground" : "text-muted-foreground",
            ].join(" ")}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

function LabeledSelect({
  label,
  onChange,
  options,
  value,
}: {
  label: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
  value: string;
}) {
  return (
    <div className="space-y-2">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <NativeSelect value={value} onChange={onChange} options={options} />
    </div>
  );
}

function NativeSelect({
  disabled,
  onChange,
  options,
  value,
}: {
  disabled?: boolean;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
  value: string;
}) {
  return (
    <label className="flex h-10 w-full min-w-[180px] items-center justify-between gap-3 rounded-xl border bg-background px-3 text-sm">
      <select
        value={value}
        onChange={(event) => onChange(event.target.value)}
        disabled={disabled}
        className="w-full appearance-none bg-transparent outline-none"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <ChevronDownIcon className="size-4 shrink-0 text-muted-foreground" />
    </label>
  );
}

const baseMediaLabels: Record<string, string> = {
  iso_a5_148x210mm: "A5 (148x210mm)",
  iso_a4_210x297mm: "A4 (210x297mm)",
  iso_a3_297x420mm: "A3 (297x420mm)",
  iso_a2_420x594mm: "A2 (420x594mm)",
  iso_a1_594x841mm: "A1 (594x841mm)",
  "na_letter_8.5x11in": "Letter (8.5x11in)",
  "na_legal_8.5x14in": "Legal (8.5x14in)",
  "na_number-10_4.125x9.5in": "No.10 Envelope (4.125x9.5in)",
  iso_dl_110x220mm: "DL Envelope (110x220mm)",
  iso_c5_162x229mm: "C5 Envelope (162x229mm)",
};

const keywordLabels: Record<string, string> = {
  all: "全部",
  auto: "自动",
  "auto-fit": "自动适应",
  color: "彩色",
  even: "偶数",
  fill: "填充纸张",
  fit: "适应纸张",
  landscape: "横向",
  monochrome: "黑白",
  none: "无缩放",
  odd: "奇数",
  "one-sided": "单面",
  portrait: "纵向",
  stationery: "普通纸",
  "thick-paper": "厚纸",
  "thick-paper-2": "厚纸 2",
  envelope: "信封",
  "label-paper": "标签纸",
  "stationery-letterhead": "信头纸",
  "coating-paper-2": "涂布纸 2",
  "coating-paper-3": "涂布纸 3",
  prepunched: "预打孔纸",
  "colored-paper": "彩色纸",
  "special-paper": "特殊纸",
  "two-sided-long-edge": "双面长边翻页",
  "two-sided-short-edge": "双面短边翻页",
};

function buildCapabilityOptions(capabilities: PrinterCapabilities | null): {
  color: SelectOption[];
  media: SelectOption[];
  mediaType: SelectOption[];
  orientation: SelectOption[];
  scaling: SelectOption[];
  sides: SelectOption[];
} {
  if (!capabilities) {
    return {
      color: [],
      media: [],
      mediaType: [],
      orientation: [],
      scaling: [],
      sides: [],
    };
  }

  return {
    color: toKeywordOptions(capabilities.color_modes_supported),
    media: toMediaOptions(capabilities.media_supported),
    mediaType: toKeywordOptions(capabilities.media_types_supported, {
      includePrinterDefault: true,
    }),
    orientation: toKeywordOptions(capabilities.orientations_supported),
    scaling: toKeywordOptions(capabilities.scalings_supported),
    sides: toKeywordOptions(capabilities.sides_supported),
  };
}

function toKeywordOptions(
  values: string[],
  options: { includePrinterDefault?: boolean } = {},
): SelectOption[] {
  const output = dedupe(values)
    .filter((value) => value.trim().length > 0)
    .map((value) => ({
      value,
      label: formatKeywordLabel(value),
    }));
  if (options.includePrinterDefault) {
    return [{ value: PRINTER_DEFAULT_VALUE, label: "使用打印机默认" }, ...output];
  }
  return output;
}

function toMediaOptions(values: string[]): SelectOption[] {
  return dedupe(values)
    .filter((value) => value.trim().length > 0 && !value.startsWith("custom_min_") && !value.startsWith("custom_max_"))
    .map((value) => ({
      value,
      label: baseMediaLabels[value] ?? formatMediaLabel(value),
    }));
}

function dedupe(values: string[]) {
  return Array.from(new Set(values.map((value) => value.trim()).filter(Boolean)));
}

function firstSupportedValue(options: SelectOption[], current: string, fallback: string) {
  if (options.some((option) => option.value === current)) return current;
  if (options.some((option) => option.value === fallback)) return fallback;
  return options[0]?.value ?? fallback;
}

function hasOption(options: SelectOption[], value: string) {
  return options.some((option) => option.value === value);
}

function buildCurrentPrintOptions({
  colorMode,
  duplexMode,
  orientation,
  pageRange,
  paperSize,
  paperType,
  scaleMode,
  capabilityOptions,
  copiesSupported,
  supportsPageRanges,
  copies,
}: {
  colorMode: ColorMode;
  duplexMode: DuplexMode;
  orientation: OrientationMode;
  pageRange: string;
  paperSize: PaperSize;
  paperType: PaperType;
  scaleMode: ScalingMode;
  capabilityOptions: ReturnType<typeof buildCapabilityOptions>;
  copiesSupported: boolean;
  supportsPageRanges: boolean;
  copies: string;
}): Record<string, unknown> {
  const printOptions: Record<string, unknown> = {};
  if (copiesSupported) {
    const parsedCopies = Number.parseInt(copies.trim(), 10);
    if (Number.isFinite(parsedCopies)) {
      printOptions.copies = Math.min(100, Math.max(1, parsedCopies));
    }
  }
  if (hasOption(capabilityOptions.color, colorMode)) {
    printOptions.printColorMode = colorMode;
  }
  if (hasOption(capabilityOptions.sides, duplexMode)) {
    printOptions.sides = duplexMode;
  }
  if (hasOption(capabilityOptions.media, paperSize)) {
    printOptions.media = paperSize;
  }
  if (paperType !== PRINTER_DEFAULT_VALUE && hasOption(capabilityOptions.mediaType, paperType)) {
    printOptions.mediaType = paperType;
  }
  if (hasOption(capabilityOptions.orientation, orientation)) {
    printOptions.orientationRequested = orientation;
  }
  if (hasOption(capabilityOptions.scaling, scaleMode)) {
    printOptions.printScaling = scaleMode;
  }

  const trimmedPageRange = pageRange.trim();
  if (supportsPageRanges && trimmedPageRange) {
    printOptions.pageRanges = trimmedPageRange;
  }
  return printOptions;
}

function getPaperDimensionsMm(paperSize: PaperSize, orientation: OrientationMode) {
  const dimensions = parseMediaDimensionsMm(paperSize) ?? fallbackPaperDimensionsMm.iso_a4_210x297mm;
  if (orientation === "landscape") {
    return {
      width: Math.max(dimensions.width, dimensions.height),
      height: Math.min(dimensions.width, dimensions.height),
    };
  }
  return {
    width: Math.min(dimensions.width, dimensions.height),
    height: Math.max(dimensions.width, dimensions.height),
  };
}

function formatPaperLabel(paperSize: PaperSize, options: SelectOption[] = []) {
  const label = options.find((option) => option.value === paperSize)?.label ?? baseMediaLabels[paperSize];
  return label?.split(" ")[0] ?? formatMediaLabel(paperSize);
}

function formatMediaLabel(value: string) {
  const dimension = parseMediaDimensionsMm(value);
  if (dimension) {
    const name = value.split("_")[1]?.toUpperCase();
    const width = formatDimension(dimension.width);
    const height = formatDimension(dimension.height);
    return name ? `${name} (${width}x${height}mm)` : `${width}x${height}mm`;
  }
  return formatKeywordLabel(value);
}

function formatKeywordLabel(value: string) {
  if (value === PRINTER_DEFAULT_VALUE) return "使用打印机默认";
  return keywordLabels[value] ?? value.replace(/[-_]/g, " ");
}

function formatDimension(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

function parseMediaDimensionsMm(value: string): { width: number; height: number } | null {
  const mmMatch = value.match(/_([0-9.]+)x([0-9.]+)mm(?:_|$)/);
  if (mmMatch) {
    return {
      width: Number(mmMatch[1]),
      height: Number(mmMatch[2]),
    };
  }

  const inchMatch = value.match(/_([0-9.]+)x([0-9.]+)in(?:_|$)/);
  if (inchMatch) {
    return {
      width: Number(inchMatch[1]) * 25.4,
      height: Number(inchMatch[2]) * 25.4,
    };
  }

  return null;
}

const fallbackPaperDimensionsMm: Record<string, { width: number; height: number }> = {
  iso_a5_148x210mm: { width: 148, height: 210 },
  iso_a4_210x297mm: { width: 210, height: 297 },
  iso_a3_297x420mm: { width: 297, height: 420 },
  iso_a2_420x594mm: { width: 420, height: 594 },
  iso_a1_594x841mm: { width: 594, height: 841 },
  "na_letter_8.5x11in": { width: 215.9, height: 279.4 },
  "na_legal_8.5x14in": { width: 215.9, height: 355.6 },
};

function estimatePdfPages(size: number) {
  if (!Number.isFinite(size) || size <= 0) return 1;
  return Math.max(1, Math.min(12, Math.ceil(size / 350_000)));
}

function buildUploadedFile(file: File, index: number): UploadedPrintFile | null {
  const lowerName = file.name.toLowerCase();
  const isPdf = file.type === "application/pdf" || lowerName.endsWith(".pdf");
  const isImage = file.type.startsWith("image/") || /\.(png|jpe?g|gif|webp|bmp)$/i.test(lowerName);
  if (!isPdf && !isImage) return null;

  return {
    id: `local-${Date.now()}-${index}-${Math.random().toString(36).slice(2)}`,
    name: file.name,
    size: file.size,
    type: isPdf ? "pdf" : "image",
    pages: isPdf ? estimatePdfPages(file.size) : 1,
    file,
    localPreviewUrl: buildObjectUrl(file),
  };
}

function isPrintableFile(file: UploadedPrintFile, templateContent: string) {
  if (file.type === "template") return templateContent.trim().length > 0;
  if (!file.file) return false;
  const lowerName = file.name.toLowerCase();
  if (file.type === "pdf") {
    return file.file.type === "application/pdf" || lowerName.endsWith(".pdf");
  }
  if (file.type === "image") {
    return file.file.type.startsWith("image/") || /\.(png|jpe?g|gif|webp|bmp)$/i.test(lowerName);
  }
  return false;
}

function buildTemplateUploadedFile(templateContent: string, dataJson: string): UploadedPrintFile {
  return {
    id: "current-template",
    name: "当前模板.pdf",
    size: 148_000 + templateContent.length + dataJson.length,
    type: "template",
    pages: 1,
    file: null,
    localPreviewUrl: null,
  };
}

function summarizeJob(job: JobResponse) {
  if (job.source_file_name) return job.source_file_name;
  if (job.job_kind === "template") return "模板打印任务";
  return job.job_kind || "打印任务";
}

function isProcessingStatus(status: string) {
  const normalized = status.toLowerCase();
  return normalized.includes("processing") || normalized.includes("pending") || normalized.includes("queued") || normalized.includes("running");
}

function isSucceededStatus(status: string) {
  const normalized = status.toLowerCase();
  return normalized.includes("succeeded") || normalized.includes("completed") || normalized.includes("done");
}

function createSyntheticSubmitEvent() {
  return {
    preventDefault: () => undefined,
  } as React.FormEvent<HTMLFormElement>;
}
