import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  ActivityIcon,
  AlertCircleIcon,
  CheckCircle2Icon,
  ClockIcon,
  CopyIcon,
  FileTextIcon,
  HardDriveIcon,
  LayersIcon,
  MoreHorizontalIcon,
  PaletteIcon,
  PlusIcon,
  PrinterIcon,
  RefreshCwIcon,
  SettingsIcon,
  ShieldAlertIcon,
  Trash2Icon,
  WifiIcon,
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  addPrinter,
  deletePrinter,
  discoverCupsPrinters,
  getCupsSettings,
  refreshPrinter,
  setDefaultPrinter,
  setPrinterEnabled,
  signClientWriteHeaders,
  testCupsConnection,
  updateCupsSettings,
  validatePrinterUri,
} from "@/features/deepprint";
import type { DeepprintController } from "@/features/deepprint/controller";
import { createPrinterDetailQueryOptions, deepprintQueryKeys } from "@/features/deepprint/queries";
import type {
  CupsConnectionTestResponse,
  DiscoveredPrinter,
  HealthResponse,
  PrinterDetail,
  PrinterInfo,
  PrinterSource,
} from "@/features/deepprint/types";
import { formatUnixSec } from "@/features/deepprint/utils";
import { printerStateLabel, statusBadgeVariant } from "@/features/deepprint/ui";
import { cn } from "@/lib/utils";

export function PrinterManagementPage({
  controller,
  showHeader = true,
}: {
  controller: DeepprintController;
  showHeader?: boolean;
}) {
  const { actions, agent, authRequiredForWrites, ui, writes } = controller;
  const queryClient = useQueryClient();
  const [serviceLoading, setServiceLoading] = useState(false);
  const [printerDetails, setPrinterDetails] = useState<Record<string, PrinterDetail>>({});
  const [refreshingPrinterId, setRefreshingPrinterId] = useState<string | null>(null);
  const [togglingPrinterId, setTogglingPrinterId] = useState<string | null>(null);
  const [defaultingPrinterId, setDefaultingPrinterId] = useState<string | null>(null);
  const [deletingPrinterId, setDeletingPrinterId] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<PrinterInfo | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [cupsBaseUrlDraft, setCupsBaseUrlDraft] = useState("");
  const [cupsSettingsLoading, setCupsSettingsLoading] = useState(false);
  const [savingCupsSettings, setSavingCupsSettings] = useState(false);
  const [testingCupsConnection, setTestingCupsConnection] = useState(false);
  const [cupsConnectionResult, setCupsConnectionResult] = useState<CupsConnectionTestResponse | null>(null);
  const [addPrinterModalOpen, setAddPrinterModalOpen] = useState(false);
  const [manualPrinterUri, setManualPrinterUri] = useState("");
  const [manualPrinterName, setManualPrinterName] = useState("");
  const [addPrinterMode, setAddPrinterMode] = useState<"discover" | "manual">("discover");
  const [addingPrinter, setAddingPrinter] = useState(false);
  const [addPrinterError, setAddPrinterError] = useState<string | null>(null);
  const [discoveringCupsPrinters, setDiscoveringCupsPrinters] = useState(false);
  const [discoveredCupsPrinters, setDiscoveredCupsPrinters] = useState<DiscoveredPrinter[]>([]);
  const [discoveryMessage, setDiscoveryMessage] = useState<string | null>(null);

  const buildAuthHeaders = async (method: string, path: string, bodyText: string) => {
    if (!authRequiredForWrites) return {};
    return signClientWriteHeaders(
      method,
      path,
      bodyText,
      writes.writeAuthToken.trim() || null,
      writes.writeAuthSecret.trim() || null,
    );
  };

  const choosePrinter = (printerId: string) => {
    writes.setCreatePrinterId(printerId);
    writes.setDirectPrinterId(printerId);
    ui.setNotice({ kind: "ok", message: "已切换打印目标" });
  };

  const loadPrinterManagement = async (showNotice = false) => {
    setServiceLoading(true);
    try {
      const [refreshResult] = await Promise.allSettled([actions.refreshAll(true)]);
      if (refreshResult?.status === "rejected") {
        throw refreshResult.reason;
      }
      if (showNotice) {
        ui.setNotice({ kind: "ok", message: "打印服务状态已刷新" });
      }
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "加载打印服务状态失败"),
      });
    } finally {
      setServiceLoading(false);
    }
  };

  const loadCupsSettingsState = async () => {
    setCupsSettingsLoading(true);
    try {
      const result = await getCupsSettings(ui.baseUrl, ui.requestTimeouts.printers);
      setCupsBaseUrlDraft(result.cups_base_url);
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "读取 CUPS 设置失败"),
      });
    } finally {
      setCupsSettingsLoading(false);
    }
  };

  const testConfiguredCupsConnection = async () => {
    const trimmed = cupsBaseUrlDraft.trim();
    if (!trimmed) return;
    setTestingCupsConnection(true);
    setCupsConnectionResult(null);
    try {
      const body = JSON.stringify({ cups_base_url: trimmed });
      const authHeaders = await buildAuthHeaders("POST", "/v1/settings/cups/test", body);
      const result = await testCupsConnection(
        ui.baseUrl,
        trimmed,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setCupsConnectionResult(result);
      ui.setNotice({ kind: "ok", message: result.message });
    } catch (error) {
      const message = getErrorMessage(error, "CUPS 连接测试失败");
      setCupsConnectionResult({
        ok: false,
        cups_base_url: trimmed,
        message,
      });
      ui.setNotice({ kind: "error", message });
    } finally {
      setTestingCupsConnection(false);
    }
  };

  const saveCupsBaseUrl = async () => {
    const trimmed = cupsBaseUrlDraft.trim();
    if (!trimmed) return;
    setSavingCupsSettings(true);
    setCupsConnectionResult(null);
    try {
      const body = JSON.stringify({ cups_base_url: trimmed });
      const authHeaders = await buildAuthHeaders("POST", "/v1/settings/cups", body);
      const result = await updateCupsSettings(
        ui.baseUrl,
        trimmed,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setCupsBaseUrlDraft(result.cups_base_url);
      ui.setNotice({ kind: "ok", message: "CUPS 地址已保存" });
      await loadPrinterManagement(false);
      if (addPrinterModalOpen) {
        await loadDiscoveredCupsPrinters();
      }
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "保存 CUPS 设置失败"),
      });
    } finally {
      setSavingCupsSettings(false);
    }
  };

  const addPrinterFromUri = async (printerUri: string, displayName: string) => {
    const trimmedUri = printerUri.trim();
    if (!trimmedUri) return;
    setAddingPrinter(true);
    setAddPrinterError(null);
    try {
      const validateBody = JSON.stringify({ uri: trimmedUri });
      const validateHeaders = await buildAuthHeaders("POST", "/v1/printers/validate", validateBody);
      const validated = await validatePrinterUri(
        ui.baseUrl,
        trimmedUri,
        ui.requestTimeouts.writes,
        validateHeaders,
      );
      const effectiveName = displayName.trim() || validated.discovered_name;
      const addBody = JSON.stringify({
        source: "manual",
        printer_uri: validated.printer_uri,
        display_name: effectiveName,
      });
      const addHeaders = await buildAuthHeaders("POST", "/v1/printers", addBody);
      const result = await addPrinter(
        ui.baseUrl,
        {
          source: "manual",
          printerUri: validated.printer_uri,
          displayName: effectiveName,
        },
        ui.requestTimeouts.writes,
        addHeaders,
      );
      choosePrinter(result.printer.id);
      setManualPrinterUri("");
      setManualPrinterName("");
      setAddPrinterModalOpen(false);
      await actions.loadPrinters();
      ui.setNotice({
        kind: "ok",
        message: result.created
          ? `已添加打印机 ${result.printer.name}`
          : `打印机 ${result.printer.name} 已在纳管列表中`,
      });
    } catch (error) {
      const message = getErrorMessage(error, "添加打印机失败");
      setAddPrinterError(message);
      ui.setNotice({ kind: "error", message });
    } finally {
      setAddingPrinter(false);
    }
  };

  const onManualAddPrinter = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await addPrinterFromUri(manualPrinterUri, manualPrinterName);
  };

  const loadDiscoveredCupsPrinters = async () => {
    setDiscoveringCupsPrinters(true);
    setAddPrinterError(null);
    setDiscoveryMessage(null);
    try {
      const result = await discoverCupsPrinters(ui.baseUrl, ui.requestTimeouts.printers);
      setDiscoveredCupsPrinters(result.printers);
      setDiscoveryMessage(result.message ?? null);
    } catch (error) {
      setAddPrinterError(getErrorMessage(error, "发现 CUPS 打印机失败"));
    } finally {
      setDiscoveringCupsPrinters(false);
    }
  };

  const addDiscoveredPrinter = async (printer: DiscoveredPrinter) => {
    setAddingPrinter(true);
    setAddPrinterError(null);
    try {
      const addBody = JSON.stringify({
        source: printer.source,
        printer_uri: printer.candidate_uri,
        display_name: printer.display_name,
      });
      const addHeaders = await buildAuthHeaders("POST", "/v1/printers", addBody);
      const result = await addPrinter(
        ui.baseUrl,
        {
          source: printer.source,
          printerUri: printer.candidate_uri,
          displayName: printer.display_name,
        },
        ui.requestTimeouts.writes,
        addHeaders,
      );
      choosePrinter(result.printer.id);
      setAddPrinterModalOpen(false);
      await actions.loadPrinters();
      ui.setNotice({
        kind: "ok",
        message: result.created
          ? `已添加打印机 ${result.printer.name}`
          : `打印机 ${result.printer.name} 已在纳管列表中`,
      });
    } catch (error) {
      const message = getErrorMessage(error, "添加打印机失败");
      setAddPrinterError(message);
      ui.setNotice({ kind: "error", message });
    } finally {
      setAddingPrinter(false);
    }
  };

  const refreshManagedPrinter = async (printerId: string) => {
    setRefreshingPrinterId(printerId);
    try {
      const path = `/v1/printers/${encodeURIComponent(printerId)}/refresh`;
      const body = JSON.stringify({});
      const authHeaders = await buildAuthHeaders("POST", path, body);
      const detail = await refreshPrinter(
        ui.baseUrl,
        printerId,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setPrinterDetails((current) => ({ ...current, [printerId]: detail }));
      await queryClient.invalidateQueries({
        queryKey: deepprintQueryKeys.printerDetail(ui.baseUrl, printerId),
      });
      await actions.loadPrinters();
      ui.setNotice({ kind: "ok", message: `已刷新打印机 ${detail.name}` });
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "刷新打印机失败"),
      });
    } finally {
      setRefreshingPrinterId(null);
    }
  };

  const toggleManagedPrinter = async (printer: PrinterInfo) => {
    setTogglingPrinterId(printer.id);
    try {
      const enabled = !printer.enabled;
      const path = `/v1/printers/${encodeURIComponent(printer.id)}/${enabled ? "enable" : "disable"}`;
      const body = JSON.stringify({});
      const authHeaders = await buildAuthHeaders("POST", path, body);
      const detail = await setPrinterEnabled(
        ui.baseUrl,
        printer.id,
        enabled,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setPrinterDetails((current) => ({ ...current, [printer.id]: detail }));
      await queryClient.invalidateQueries({
        queryKey: deepprintQueryKeys.printerDetail(ui.baseUrl, printer.id),
      });
      await actions.loadPrinters();
      ui.setNotice({
        kind: "ok",
        message: enabled ? `已启用打印机 ${printer.name}` : `已停用打印机 ${printer.name}`,
      });
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "更新打印机状态失败"),
      });
    } finally {
      setTogglingPrinterId(null);
    }
  };

  const makeDefaultPrinter = async (printer: PrinterInfo) => {
    setDefaultingPrinterId(printer.id);
    try {
      const path = `/v1/printers/${encodeURIComponent(printer.id)}/set-default`;
      const body = JSON.stringify({});
      const authHeaders = await buildAuthHeaders("POST", path, body);
      const detail = await setDefaultPrinter(
        ui.baseUrl,
        printer.id,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setPrinterDetails((current) => ({ ...current, [printer.id]: detail }));
      await queryClient.invalidateQueries({
        queryKey: deepprintQueryKeys.printerDetail(ui.baseUrl, printer.id),
      });
      await actions.loadPrinters();
      ui.setNotice({ kind: "ok", message: `已设 ${printer.name} 为默认打印机` });
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "设置默认打印机失败"),
      });
    } finally {
      setDefaultingPrinterId(null);
    }
  };

  const confirmDeletePrinter = async () => {
    if (!pendingDelete) return;
    setDeletingPrinterId(pendingDelete.id);
    try {
      const path = `/v1/printers/${encodeURIComponent(pendingDelete.id)}`;
      const body = JSON.stringify({});
      const authHeaders = await buildAuthHeaders("DELETE", path, body);
      await deletePrinter(
        ui.baseUrl,
        pendingDelete.id,
        ui.requestTimeouts.writes,
        authHeaders,
      );
      setPrinterDetails((current) => {
        const next = { ...current };
        delete next[pendingDelete.id];
        return next;
      });
      if (writes.createPrinterId === pendingDelete.id) writes.setCreatePrinterId("");
      if (writes.directPrinterId === pendingDelete.id) writes.setDirectPrinterId("");
      ui.setNotice({
        kind: "ok",
        message: `已删除打印机 ${pendingDelete.name}`,
      });
      setPendingDelete(null);
      await actions.loadPrinters();
    } catch (error) {
      ui.setNotice({
        kind: "error",
        message: getErrorMessage(error, "删除打印机失败"),
      });
    } finally {
      setDeletingPrinterId(null);
    }
  };

  const openAddPrinterModal = () => {
    setAddPrinterError(null);
    setDiscoveredCupsPrinters([]);
    setDiscoveryMessage(null);
    setAddPrinterMode("discover");
    setAddPrinterModalOpen(true);
    void loadDiscoveredCupsPrinters();
  };

  const openSettingsPanel = () => {
    setCupsConnectionResult(null);
    setSettingsOpen(true);
    void loadCupsSettingsState();
  };

  useEffect(() => {
    void loadPrinterManagement(false);
  }, [ui.baseUrl]);

  useEffect(() => {
    agent.printers.forEach((printer) => {
      if (printerDetails[printer.id]) return;
      void queryClient
        .fetchQuery({
          ...createPrinterDetailQueryOptions({
            baseUrl: ui.baseUrl,
            printerId: printer.id,
            timeoutMs: ui.requestTimeouts.printers,
          }),
        })
        .then((detail) => {
          setPrinterDetails((current) => ({ ...current, [printer.id]: detail }));
        })
        .catch(() => undefined);
    });
  }, [agent.printers, printerDetails, queryClient, ui.baseUrl, ui.requestTimeouts.printers]);

  const selectedPrinterId = writes.createPrinterId || writes.directPrinterId;
  const printersDescription =
    agent.printersNote && agent.printersNote !== "-"
      ? agent.printersNote
      : "管理已接入的打印机及其默认状态。";
  return (
    <div className="animate-in space-y-6 duration-300 fade-in slide-in-from-bottom-2">
      <SectionHeader
        title={showHeader ? "打印机管理" : "打印服务中心"}
        description="连接 CUPS，纳管打印机，并确认每台设备的状态与能力。"
        health={agent.health}
        onAddPrinter={openAddPrinterModal}
        onOpenSettings={openSettingsPanel}
      />

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <ServiceMetricCard
          title="服务状态"
          value={
            <span className="flex items-center gap-2">
              <StatusDot status={agent.health?.status} />
              {serverStatusLabel(agent.health)}
            </span>
          }
          caption={
            agent.health
              ? `${agent.health.backend_name ?? "unknown"} / ${agent.health.render_engine}`
              : "正在读取服务状态"
          }
          icon={<ActivityIcon className="size-5" />}
          tone={agent.health?.status === "ok" ? "success" : "warning"}
        />
        <ServiceMetricCard
          title="队列中任务"
          value={`${agent.health?.queue_length ?? 0} 个`}
          caption={`渲染 ${agent.health?.rendering_jobs ?? 0} · 提交 ${agent.health?.submitting_jobs ?? 0} · 打印 ${agent.health?.printing_jobs ?? 0}`}
          icon={<LayersIcon className="size-5" />}
        />
        <ServiceMetricCard
          title="异常待处理"
          value={(agent.health?.needs_attention_jobs ?? 0) > 0 ? `${agent.health?.needs_attention_jobs} 个` : "无异常"}
          caption={serverSummaryText(agent.health)}
          icon={<ShieldAlertIcon className="size-5" />}
          tone={(agent.health?.needs_attention_jobs ?? 0) > 0 ? "danger" : "neutral"}
        />
        <ServiceMetricCard
          title="已稳定运行"
          value={formatDuration(agent.health?.uptime_seconds)}
          caption={`版本 ${agent.health?.version ?? "读取中"}`}
          icon={<ClockIcon className="size-5" />}
        />
      </section>

      <Card className="overflow-hidden">
        <CardHeader className="gap-3 border-b pb-4">
          <CardTitle>设备列表</CardTitle>
          <CardDescription>{printersDescription}</CardDescription>
          <CardAction className="col-start-1 row-start-3 mt-2 flex w-full flex-col gap-2 justify-self-stretch sm:col-start-2 sm:row-span-2 sm:row-start-1 sm:mt-0 sm:w-auto sm:flex-row sm:justify-self-end">
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="w-full sm:w-auto"
              disabled={serviceLoading || agent.refreshingAll}
              onClick={() => void loadPrinterManagement(true)}
            >
              <RefreshCwIcon data-icon="inline-start" />
              刷新状态
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent className="pt-4">
          <PrinterTable
            defaultingPrinterId={defaultingPrinterId}
            deletingPrinterId={deletingPrinterId}
            details={printerDetails}
            loading={agent.loadingPrinters}
            onAddPrinter={openAddPrinterModal}
            onChoosePrinter={choosePrinter}
            onDeletePrinter={setPendingDelete}
            onRefreshPrinter={(printerId) => void refreshManagedPrinter(printerId)}
            onSetDefaultPrinter={(printer) => void makeDefaultPrinter(printer)}
            onTogglePrinter={(printer) => void toggleManagedPrinter(printer)}
            printers={agent.printers}
            refreshingPrinterId={refreshingPrinterId}
            selectedPrinterId={selectedPrinterId}
            togglingPrinterId={togglingPrinterId}
          />
        </CardContent>
      </Card>

      <Sheet open={addPrinterModalOpen} onOpenChange={setAddPrinterModalOpen}>
        <SheetContent className="w-full max-w-none overflow-hidden data-[side=right]:w-full data-[side=right]:sm:max-w-2xl">
          <SheetHeader>
            <SheetTitle>添加打印机</SheetTitle>
            <SheetDescription>从当前打印服务发现设备，或手动输入已知打印机 URI。</SheetDescription>
          </SheetHeader>
          <Tabs
            className="min-h-0 flex-1 gap-0 overflow-hidden"
            value={addPrinterMode}
            onValueChange={(value) => setAddPrinterMode(value as "discover" | "manual")}
          >
            <div className="border-b bg-muted/20 px-4 pt-2">
              <TabsList variant="line" className="w-full justify-start gap-4 sm:w-auto">
                <TabsTrigger value="discover" className="flex-1 sm:flex-none">
                  从当前 CUPS 发现
                </TabsTrigger>
                <TabsTrigger value="manual" className="flex-1 sm:flex-none">
                  手动输入地址
                </TabsTrigger>
              </TabsList>
            </div>

            <div className="min-h-0 flex-1 overflow-y-auto px-4 py-5">
              <TabsContent value="discover" className="m-0 space-y-4">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                  <p className="text-sm leading-6 text-muted-foreground">
                    读取当前 DeepPrint 后端已连接的 CUPS 打印机，选择后添加到设备列表。
                  </p>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="w-full sm:w-auto"
                    disabled={discoveringCupsPrinters}
                    onClick={() => void loadDiscoveredCupsPrinters()}
                  >
                    <RefreshCwIcon data-icon="inline-start" className={discoveringCupsPrinters ? "animate-spin" : undefined} />
                    {discoveringCupsPrinters ? "发现中..." : "重新扫描"}
                  </Button>
                </div>

                {addPrinterError ? <InlineError message={addPrinterError} /> : null}

                <div className="overflow-hidden rounded-xl border bg-background">
                  {discoveredCupsPrinters.length ? (
                    <div className="divide-y">
                      {discoveredCupsPrinters.map((printer) => (
                        <div
                          key={printer.candidate_uri}
                          className="flex flex-col gap-3 p-4 transition-colors hover:bg-muted/30 sm:flex-row sm:items-center sm:justify-between"
                        >
                          <div className="flex min-w-0 items-start gap-3">
                            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground">
                              <PrinterIcon className="size-5" />
                            </div>
                            <div className="min-w-0">
                              <div className="flex flex-wrap items-center gap-2 text-sm font-medium">
                                <span className="truncate">{printer.display_name}</span>
                                {printer.managed_printer_id ? <Badge variant="secondary">已在列表</Badge> : null}
                              </div>
                              <div className="ui-selectable mt-1 max-w-full truncate font-mono text-xs text-muted-foreground" title={printer.candidate_uri}>
                                {printer.candidate_uri}
                              </div>
                            </div>
                          </div>
                          <Button
                            type="button"
                            size="sm"
                            className="w-full sm:w-auto"
                            disabled={addingPrinter || Boolean(printer.managed_printer_id)}
                            onClick={() => void addDiscoveredPrinter(printer)}
                          >
                            {printer.managed_printer_id ? "已添加" : "添加"}
                          </Button>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="px-4 py-10 text-center text-sm text-muted-foreground">
                      <PrinterIcon className="mx-auto mb-3 size-9 text-muted-foreground/50" />
                      <div className="font-medium text-foreground">
                        {discoveringCupsPrinters ? "正在读取打印机列表..." : "当前没有可添加的 CUPS 打印机"}
                      </div>
                      <p className="mx-auto mt-2 max-w-md leading-6">
                        {discoveringCupsPrinters
                          ? "正在向后端查询当前 CUPS 已声明的打印机。"
                          : discoveryMessage || "请确认 CUPS 中已有打印机，且该打印机允许共享或能被后端读取。"}
                      </p>
                    </div>
                  )}
                </div>
              </TabsContent>

              <TabsContent value="manual" className="m-0">
                <form className="space-y-5" onSubmit={onManualAddPrinter}>
                  <div className="rounded-xl border border-sky-200 bg-sky-50/70 px-4 py-3 text-sm leading-6 text-sky-900">
                    <AlertCircleIcon className="mr-2 inline size-4 align-[-2px] text-sky-700" />
                    如果你知道打印机的确切 URI，可以在这里手动添加。后端会先校验连接与能力，再纳入设备列表。
                  </div>
                  {addPrinterError ? <InlineError message={addPrinterError} /> : null}
                  <div className="grid gap-4">
                    <div className="space-y-2">
                      <Label htmlFor="manual-printer-uri">打印机 URI</Label>
                      <Input
                        id="manual-printer-uri"
                        placeholder="例如：ipps://printer.local/ipp/print 或 socket://192.168.1.50:9100"
                        value={manualPrinterUri}
                        onChange={(event) => setManualPrinterUri(event.target.value)}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="manual-printer-name">显示名称（可选）</Label>
                      <Input
                        id="manual-printer-name"
                        placeholder="留空则自动使用设备名称"
                        value={manualPrinterName}
                        onChange={(event) => setManualPrinterName(event.target.value)}
                      />
                    </div>
                  </div>
                  <div className="flex flex-col-reverse gap-2 border-t pt-4 sm:flex-row sm:justify-end">
                    <Button type="button" variant="outline" onClick={() => setAddPrinterModalOpen(false)}>
                      取消
                    </Button>
                    <Button type="submit" disabled={!manualPrinterUri.trim() || addingPrinter}>
                      <PlusIcon data-icon="inline-start" />
                      {addingPrinter ? "校验中..." : "校验并添加"}
                    </Button>
                  </div>
                </form>
              </TabsContent>
            </div>
          </Tabs>
          <SheetFooter>
            <Button type="button" variant="outline" onClick={() => setAddPrinterModalOpen(false)}>
              关闭
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      <Sheet open={settingsOpen} onOpenChange={setSettingsOpen}>
        <SheetContent className="w-full max-w-none overflow-hidden data-[side=right]:w-full data-[side=right]:sm:max-w-lg">
          <SheetHeader>
            <SheetTitle>服务设置</SheetTitle>
            <SheetDescription>配置 DeepPrint 后端当前连接的 CUPS 服务地址。</SheetDescription>
          </SheetHeader>
          <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-4">
            <div className="rounded-xl border bg-muted/20 p-4">
              <div className="flex items-start gap-3">
                <div className="rounded-lg border bg-background p-2 text-muted-foreground">
                  <HardDriveIcon className="size-4" />
                </div>
                <div className="min-w-0">
                  <div className="text-sm font-medium">当前 CUPS 地址</div>
                  <div className="ui-selectable mt-1 truncate font-mono text-xs text-muted-foreground" title={cupsBaseUrlDraft || undefined}>
                    {cupsSettingsLoading ? "读取中..." : cupsBaseUrlDraft || "未配置"}
                  </div>
                </div>
              </div>
              <div className="mt-3 grid gap-2 sm:grid-cols-3">
                <SummaryPill label="打印后端" value={agent.health?.backend_name ?? "unknown"} />
                <SummaryPill label="渲染引擎" value={agent.health?.render_engine ?? "unknown"} />
                <SummaryPill label="缓存" value={`${agent.health?.render_cache_entries ?? 0} 条`} />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="settings-cups-base-url">CUPS Base URL</Label>
              <Input
                id="settings-cups-base-url"
                placeholder="http://127.0.0.1:631"
                value={cupsBaseUrlDraft}
                onChange={(event) => setCupsBaseUrlDraft(event.target.value)}
              />
              <p className="text-xs leading-5 text-muted-foreground">
                这里填写的是 DeepPrint 服务端可访问的地址。宿主机部署通常可以用
                <code className="mx-1 rounded bg-muted px-1 py-0.5">http://127.0.0.1:631</code>
                ，Docker Compose 常见为
                <code className="mx-1 rounded bg-muted px-1 py-0.5">http://cups:631</code>
                。
              </p>
            </div>

            <div className="grid gap-2 sm:grid-cols-2">
              <Button
                type="button"
                variant="outline"
                disabled={testingCupsConnection || cupsSettingsLoading || !cupsBaseUrlDraft.trim()}
                onClick={() => void testConfiguredCupsConnection()}
              >
                <WifiIcon data-icon="inline-start" />
                {testingCupsConnection ? "测试中..." : "测试连接"}
              </Button>
              <Button
                type="button"
                disabled={savingCupsSettings || cupsSettingsLoading || !cupsBaseUrlDraft.trim()}
                onClick={() => void saveCupsBaseUrl()}
              >
                {savingCupsSettings ? "保存中..." : "保存设置"}
              </Button>
            </div>

            {cupsConnectionResult ? (
              <div
                className={cn(
                  "rounded-xl border px-4 py-3 text-sm",
                  cupsConnectionResult.ok
                    ? "border-emerald-200 bg-emerald-50 text-emerald-900"
                    : "border-destructive/30 bg-destructive/10 text-destructive",
                )}
              >
                {cupsConnectionResult.message}
              </div>
            ) : null}
          </div>
          <SheetFooter>
            <Button type="button" variant="outline" onClick={() => setSettingsOpen(false)}>
              关闭
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      {pendingDelete ? (
        <div className="fixed inset-0 z-[60] flex items-center justify-center bg-background/80 p-4 backdrop-blur-sm">
          <Card className="w-full max-w-md border shadow-2xl">
            <CardHeader>
              <CardTitle>删除打印机</CardTitle>
              <CardDescription>
                确认删除“{pendingDelete.name}”？如果还有进行中的打印任务，后端会阻止删除。
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  disabled={Boolean(deletingPrinterId)}
                  onClick={() => setPendingDelete(null)}
                >
                  取消
                </Button>
                <Button
                  type="button"
                  variant="destructive"
                  disabled={Boolean(deletingPrinterId)}
                  onClick={() => void confirmDeletePrinter()}
                >
                  删除
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      ) : null}
    </div>
  );
}

function SectionHeader({
  description,
  health,
  onAddPrinter,
  onOpenSettings,
  title,
}: {
  description: string;
  health: HealthResponse | null | undefined;
  onAddPrinter: () => void;
  onOpenSettings: () => void;
  title: string;
}) {
  return (
    <div className="relative overflow-hidden rounded-2xl border bg-card p-4 shadow-sm sm:p-6">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_12%_20%,rgba(14,165,233,0.16),transparent_34%),radial-gradient(circle_at_88%_8%,rgba(16,185,129,0.14),transparent_32%),linear-gradient(135deg,rgba(14,165,233,0.08),transparent_48%,rgba(16,185,129,0.08))]" />
      <div className="relative flex flex-col gap-5 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex min-w-0 gap-4">
          <div className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm">
            <PrinterIcon className="size-5" />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="font-heading text-xl font-semibold tracking-tight sm:text-2xl">{title}</h1>
              <Badge variant={statusBadgeVariant(health?.status)}>
                <StatusDot status={health?.status} />
                {serverStatusLabel(health)}
              </Badge>
            </div>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p>
          </div>
        </div>
        <div className="grid gap-2 sm:grid-cols-2 lg:flex lg:shrink-0">
          <Button type="button" variant="outline" className="w-full lg:w-auto" onClick={onOpenSettings}>
            <SettingsIcon data-icon="inline-start" />
            设置
          </Button>
          <Button type="button" className="w-full lg:w-auto" onClick={onAddPrinter}>
            <PlusIcon data-icon="inline-start" />
            添加打印机
          </Button>
        </div>
      </div>
    </div>
  );
}

function ServiceMetricCard({
  caption,
  icon,
  title,
  tone = "default",
  value,
}: {
  caption: string;
  icon: ReactNode;
  title: string;
  tone?: "default" | "success" | "warning" | "danger" | "neutral";
  value: ReactNode;
}) {
  return (
    <div
      className={cn(
        "rounded-xl border bg-card p-4 shadow-sm",
        tone === "success" ? "border-emerald-200 bg-emerald-50/60" : "",
        tone === "warning" ? "border-amber-200 bg-amber-50/60" : "",
        tone === "danger" ? "border-destructive/30 bg-destructive/10" : "",
      )}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="text-sm font-medium text-muted-foreground">{title}</div>
        <div
          className={cn(
            "rounded-lg border bg-background p-2 text-muted-foreground",
            tone === "success" ? "text-emerald-700" : "",
            tone === "warning" ? "text-amber-700" : "",
            tone === "danger" ? "text-destructive" : "",
          )}
        >
          {icon}
        </div>
      </div>
      <div className="mt-4 text-2xl font-semibold tracking-tight">{value}</div>
      <div className="mt-1 min-h-8 text-xs leading-4 text-muted-foreground">{caption}</div>
    </div>
  );
}

function PrinterTable({
  defaultingPrinterId,
  deletingPrinterId,
  details,
  loading,
  onAddPrinter,
  onChoosePrinter,
  onDeletePrinter,
  onRefreshPrinter,
  onSetDefaultPrinter,
  onTogglePrinter,
  printers,
  refreshingPrinterId,
  selectedPrinterId,
  togglingPrinterId,
}: {
  defaultingPrinterId: string | null;
  deletingPrinterId: string | null;
  details: Record<string, PrinterDetail>;
  loading: boolean;
  onAddPrinter: () => void;
  onChoosePrinter: (printerId: string) => void;
  onDeletePrinter: (printer: PrinterInfo) => void;
  onRefreshPrinter: (printerId: string) => void;
  onSetDefaultPrinter: (printer: PrinterInfo) => void;
  onTogglePrinter: (printer: PrinterInfo) => void;
  printers: PrinterInfo[];
  refreshingPrinterId: string | null;
  selectedPrinterId: string;
  togglingPrinterId: string | null;
}) {
  if (!printers.length) {
    return (
      <div className="rounded-xl border border-dashed bg-muted/20 px-4 py-12 text-center">
        <PrinterIcon className="mx-auto mb-4 size-10 text-muted-foreground/60" />
        <div className="font-medium">{loading ? "正在加载打印机..." : "暂无托管打印机"}</div>
        <p className="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">
          可以从当前 CUPS 发现设备，也可以手动输入已知的打印机 URI。
        </p>
        <Button type="button" className="mt-4" onClick={onAddPrinter}>
          <PlusIcon data-icon="inline-start" />
          添加打印机
        </Button>
      </div>
    );
  }

  const actionProps = {
    defaultingPrinterId,
    deletingPrinterId,
    onChoosePrinter,
    onDeletePrinter,
    onRefreshPrinter,
    onSetDefaultPrinter,
    onTogglePrinter,
    refreshingPrinterId,
    selectedPrinterId,
    togglingPrinterId,
  };

  return (
    <>
      <div className="hidden overflow-hidden rounded-xl border md:block">
        <Table>
          <TableHeader>
            <TableRow className="bg-muted/30">
              <TableHead>打印机</TableHead>
              <TableHead>状态</TableHead>
              <TableHead>核心能力</TableHead>
              <TableHead>最近验证</TableHead>
              <TableHead className="text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {printers.map((printer) => {
              const detail = details[printer.id];
              return (
                <TableRow key={printer.id} className={cn(!printer.enabled ? "opacity-65" : "")}>
                  <TableCell className="max-w-[360px]">
                    <PrinterIdentity
                      printer={printer}
                      selected={printer.id === selectedPrinterId}
                      onChoosePrinter={onChoosePrinter}
                    />
                  </TableCell>
                  <TableCell className="align-middle">
                    <PrinterStatus printer={printer} />
                  </TableCell>
                  <TableCell>
                    <CapabilityChips detail={detail} />
                  </TableCell>
                  <TableCell className="whitespace-nowrap text-sm text-muted-foreground">
                    {formatUnixSec(printer.last_validated_at)}
                  </TableCell>
                  <TableCell className="text-right">
                    <PrinterActionsMenu printer={printer} {...actionProps} />
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </div>

      <div className="space-y-3 md:hidden">
        {printers.map((printer) => (
          <PrinterMobileCard
            key={printer.id}
            detail={details[printer.id]}
            printer={printer}
            {...actionProps}
          />
        ))}
      </div>
    </>
  );
}

function PrinterIdentity({
  onChoosePrinter,
  printer,
  selected,
}: {
  onChoosePrinter: (printerId: string) => void;
  printer: PrinterInfo;
  selected: boolean;
}) {
  return (
    <div className="flex min-w-0 items-start gap-3">
      <button
        type="button"
        className={cn(
          "mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-xl border transition-colors",
          printer.is_default ? "bg-primary/10 text-primary" : "bg-muted/40 text-muted-foreground",
        )}
        onClick={() => onChoosePrinter(printer.id)}
        title="选为打印目标"
      >
        <PrinterIcon className="size-5" />
      </button>
      <div className="min-w-0">
        <button
          type="button"
          className="ui-selectable max-w-full truncate text-left font-medium hover:text-primary"
          onClick={() => onChoosePrinter(printer.id)}
        >
          {printer.name}
        </button>
        <div className="mt-1 flex flex-wrap gap-1.5">
          {selected ? <Badge variant="secondary">打印目标</Badge> : null}
          {printer.is_default ? <Badge variant="outline">默认设备</Badge> : null}
          {!printer.enabled ? <Badge variant="outline">已停用</Badge> : null}
          <Badge variant="outline">{printerSourceLabel(printer.source)}</Badge>
        </div>
        <div className="ui-selectable mt-1 max-w-full truncate font-mono text-xs text-muted-foreground" title={printer.uri}>
          {printer.uri}
        </div>
      </div>
    </div>
  );
}

function PrinterStatus({ printer }: { printer: PrinterInfo }) {
  return (
    <div className="flex flex-col items-start gap-1">
      <Badge variant={printer.enabled ? statusBadgeVariant(printer.state) : "outline"}>
        <StatusDot status={printer.enabled ? printer.state : "disabled"} />
        {printerStateLabel(printer)}
      </Badge>
      {printer.state_message ? (
        <div className="max-w-56 truncate text-xs text-muted-foreground" title={printer.state_message}>
          {printer.state_message}
        </div>
      ) : null}
    </div>
  );
}

function PrinterMobileCard({
  defaultingPrinterId,
  deletingPrinterId,
  detail,
  onChoosePrinter,
  onDeletePrinter,
  onRefreshPrinter,
  onSetDefaultPrinter,
  onTogglePrinter,
  printer,
  refreshingPrinterId,
  selectedPrinterId,
  togglingPrinterId,
}: PrinterActionProps & {
  detail: PrinterDetail | undefined;
  printer: PrinterInfo;
}) {
  const selected = printer.id === selectedPrinterId;
  return (
    <div className={cn("rounded-xl border bg-card p-4 shadow-sm", !printer.enabled ? "opacity-70" : "")}>
      <div className="flex items-start justify-between gap-3">
        <PrinterIdentity printer={printer} selected={selected} onChoosePrinter={onChoosePrinter} />
        <PrinterActionsMenu
          defaultingPrinterId={defaultingPrinterId}
          deletingPrinterId={deletingPrinterId}
          onChoosePrinter={onChoosePrinter}
          onDeletePrinter={onDeletePrinter}
          onRefreshPrinter={onRefreshPrinter}
          onSetDefaultPrinter={onSetDefaultPrinter}
          onTogglePrinter={onTogglePrinter}
          printer={printer}
          refreshingPrinterId={refreshingPrinterId}
          selectedPrinterId={selectedPrinterId}
          togglingPrinterId={togglingPrinterId}
        />
      </div>
      <div className="mt-4 grid gap-3">
        <div className="flex items-center justify-between gap-3 rounded-lg border bg-muted/20 px-3 py-2">
          <span className="text-xs text-muted-foreground">状态</span>
          <PrinterStatus printer={printer} />
        </div>
        <div className="rounded-lg border bg-muted/20 px-3 py-2">
          <div className="mb-2 text-xs text-muted-foreground">核心能力</div>
          <CapabilityChips detail={detail} />
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs">
          <SummaryPill label="最近验证" value={formatUnixSec(printer.last_validated_at)} />
          <SummaryPill label="最近发现" value={formatUnixSec(printer.last_seen_at)} />
        </div>
        <div className="grid grid-cols-2 gap-2">
          <Button
            type="button"
            variant={selected ? "secondary" : "outline"}
            size="sm"
            disabled={selected}
            onClick={() => onChoosePrinter(printer.id)}
          >
            {selected ? "打印目标" : "选为打印目标"}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={refreshingPrinterId === printer.id}
            onClick={() => onRefreshPrinter(printer.id)}
          >
            刷新状态
          </Button>
        </div>
      </div>
    </div>
  );
}

type PrinterActionProps = {
  defaultingPrinterId: string | null;
  deletingPrinterId: string | null;
  onChoosePrinter: (printerId: string) => void;
  onDeletePrinter: (printer: PrinterInfo) => void;
  onRefreshPrinter: (printerId: string) => void;
  onSetDefaultPrinter: (printer: PrinterInfo) => void;
  onTogglePrinter: (printer: PrinterInfo) => void;
  refreshingPrinterId: string | null;
  selectedPrinterId: string;
  togglingPrinterId: string | null;
};

function PrinterActionsMenu({
  defaultingPrinterId,
  deletingPrinterId,
  onChoosePrinter,
  onDeletePrinter,
  onRefreshPrinter,
  onSetDefaultPrinter,
  onTogglePrinter,
  printer,
  refreshingPrinterId,
  selectedPrinterId,
  togglingPrinterId,
}: PrinterActionProps & { printer: PrinterInfo }) {
  const selected = printer.id === selectedPrinterId;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            className="text-muted-foreground data-[state=open]:bg-muted"
          />
        }
      >
        <MoreHorizontalIcon />
        <span className="sr-only">打开打印机操作菜单</span>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        <DropdownMenuGroup>
          <DropdownMenuLabel>设备操作</DropdownMenuLabel>
          {!selected ? (
            <DropdownMenuItem onClick={() => onChoosePrinter(printer.id)}>
              <CheckCircle2Icon />
              选为打印目标
            </DropdownMenuItem>
          ) : null}
          {!printer.is_default ? (
            <DropdownMenuItem
              disabled={defaultingPrinterId === printer.id}
              onClick={() => onSetDefaultPrinter(printer)}
            >
              <PrinterIcon />
              设为默认设备
            </DropdownMenuItem>
          ) : null}
          <DropdownMenuItem disabled={refreshingPrinterId === printer.id} onClick={() => onRefreshPrinter(printer.id)}>
            <RefreshCwIcon />
            刷新状态
          </DropdownMenuItem>
          <DropdownMenuItem
            disabled={togglingPrinterId === printer.id || (printer.enabled && printer.is_default)}
            onClick={() => onTogglePrinter(printer)}
          >
            {printer.enabled ? <AlertCircleIcon /> : <CheckCircle2Icon />}
            {printer.enabled && printer.is_default ? "默认设备不能停用" : printer.enabled ? "停用设备" : "启用设备"}
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          variant="destructive"
          disabled={deletingPrinterId === printer.id}
          onClick={() => onDeletePrinter(printer)}
        >
          <Trash2Icon />
          移除设备
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function CapabilityChips({ detail }: { detail: PrinterDetail | undefined }) {
  if (!detail) {
    return <span className="text-xs text-muted-foreground">能力加载中</span>;
  }

  const chips = buildCapabilityChips(detail);
  if (!chips.length) {
    return <span className="text-xs text-muted-foreground">CUPS 未声明核心能力</span>;
  }

  const visibleChips = chips.slice(0, 2);
  const hiddenChips = chips.slice(2);

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {visibleChips.map((chip) => (
        <CapabilityChip key={chip.key} chip={chip} />
      ))}
      {hiddenChips.length ? (
        <Tooltip>
          <TooltipTrigger
            render={
              <span className="inline-flex cursor-help items-center justify-center rounded-full border bg-muted/60 px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted" />
            }
          >
            +{hiddenChips.length}
          </TooltipTrigger>
          <TooltipContent side="top" align="center" className="block max-w-64 px-3 py-2">
            <div className="mb-1 border-b border-background/20 pb-1 font-medium text-background/80">其他能力</div>
            <div className="space-y-1">
              {hiddenChips.map((chip) => (
                <div key={chip.key} className="flex items-center gap-1.5 whitespace-nowrap">
                  <CapabilityIcon icon={chip.icon} className="size-3 text-background/70" />
                  <span>{chip.label}</span>
                </div>
              ))}
            </div>
          </TooltipContent>
        </Tooltip>
      ) : null}
    </div>
  );
}

type CapabilityChipModel = {
  icon: "color" | "duplex" | "media" | "copies" | "page-ranges" | "orientation" | "scaling";
  key: string;
  label: string;
  tone: "primary" | "success" | "muted";
};

function CapabilityChip({ chip }: { chip: CapabilityChipModel }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-1 text-xs",
        chip.tone === "primary" ? "border-sky-200 bg-sky-50 text-sky-800" : "",
        chip.tone === "success" ? "border-emerald-200 bg-emerald-50 text-emerald-800" : "",
        chip.tone === "muted" ? "bg-muted/50 text-muted-foreground" : "",
      )}
    >
      <CapabilityIcon icon={chip.icon} className="size-3" />
      {chip.label}
    </span>
  );
}

function CapabilityIcon({
  className,
  icon,
}: {
  className?: string;
  icon: CapabilityChipModel["icon"];
}) {
  switch (icon) {
    case "color":
      return <PaletteIcon className={className} />;
    case "duplex":
      return <LayersIcon className={className} />;
    case "copies":
      return <CopyIcon className={className} />;
    case "page-ranges":
    case "orientation":
      return <FileTextIcon className={className} />;
    case "media":
    case "scaling":
      return <HardDriveIcon className={className} />;
  }
}

function SummaryPill({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-lg border bg-background px-3 py-2">
      <div className="text-[11px] text-muted-foreground">{label}</div>
      <div className="mt-1 truncate text-xs font-medium" title={value}>
        {value}
      </div>
    </div>
  );
}

function InlineError({ message }: { message: string }) {
  return (
    <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
      {message}
    </div>
  );
}

function StatusDot({ status }: { status: string | null | undefined }) {
  const normalized = (status ?? "").toLowerCase();
  const tone =
    normalized.includes("idle") || normalized === "ok" || normalized === "succeeded"
      ? "success"
      : normalized.includes("process") || normalized.includes("busy") || normalized.includes("print")
        ? "progress"
        : normalized.includes("failed") || normalized.includes("error") || normalized.includes("stop")
          ? "danger"
          : "muted";

  return (
    <span className="relative inline-flex size-2.5 shrink-0">
      {tone === "success" ? <span className="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-60" /> : null}
      <span
        className={cn(
          "relative inline-flex size-2.5 rounded-full",
          tone === "success" ? "bg-emerald-500" : "",
          tone === "progress" ? "bg-sky-500" : "",
          tone === "danger" ? "bg-destructive" : "",
          tone === "muted" ? "bg-muted-foreground/40" : "",
        )}
      />
    </span>
  );
}

function serverStatusLabel(health: { status: string } | null | undefined) {
  if (!health) return "加载中";
  return health.status === "ok" ? "运行中" : health.status || "未知";
}

function serverSummaryText(
  health: Pick<
    HealthResponse,
    "status" | "needs_attention_jobs" | "rendering_jobs" | "submitting_jobs" | "printing_jobs"
  > | null | undefined,
) {
  if (!health) return "正在读取 Print Server 状态。";
  if ((health.needs_attention_jobs ?? 0) > 0) {
    return `当前有 ${health.needs_attention_jobs} 个任务状态待确认，建议进入打印历史查看。`;
  }
  if (health.status !== "ok") {
    return "Print Server 状态异常，建议检查任务队列、后端与缓存信息。";
  }
  if (health.rendering_jobs > 0 || (health.submitting_jobs ?? 0) > 0 || health.printing_jobs > 0) {
    return "Print Server 运行中，当前正在处理打印任务。";
  }
  return "Print Server 运行中，负责打印渲染、提交与任务状态追踪。";
}

function formatDuration(seconds: number | null | undefined) {
  if (!seconds || !Number.isFinite(seconds) || seconds <= 0) return "刚启动";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return `${hours} 小时 ${minutes} 分钟`;
  if (minutes > 0) return `${minutes} 分钟`;
  return `${Math.max(1, Math.floor(seconds))} 秒`;
}

function buildCapabilityChips(printer: PrinterDetail) {
  const { capabilities } = printer;
  const chips: CapabilityChipModel[] = [];

  const colorModes = capabilities.color_modes_supported.filter(Boolean);
  if (colorModes.length || capabilities.color_supported !== null) {
    const normalizedColorModes = colorModes.map((value) => value.toLowerCase());
    const supportsColor =
      capabilities.color_supported === true || normalizedColorModes.some((value) => value.includes("color"));
    const supportsMonochrome =
      capabilities.color_supported === false ||
      normalizedColorModes.some((value) => value.includes("monochrome") || value.includes("black"));
    chips.push({
      icon: "color",
      key: "color",
      label: supportsColor && supportsMonochrome ? "彩色/黑白" : supportsColor ? "彩色" : "黑白",
      tone: supportsColor ? "primary" : "muted",
    });
  }

  const sides = capabilities.sides_supported.filter(Boolean);
  if (sides.length) {
    const supportsDuplex = sides.some((value) => value.toLowerCase().startsWith("two-sided"));
    chips.push({
      icon: "duplex",
      key: "duplex",
      label: supportsDuplex ? "双面" : "单面",
      tone: supportsDuplex ? "success" : "muted",
    });
  }

  const media = capabilities.media_supported.filter(
    (value) => value.trim() && !value.startsWith("custom_min_") && !value.startsWith("custom_max_"),
  );
  if (media.length) {
    chips.push({
      icon: "media",
      key: "media",
      label: media.length > 1 ? `${formatMediaLabel(media[0])} 等 ${media.length} 种` : formatMediaLabel(media[0]),
      tone: "muted",
    });
  }

  const copiesMax = capabilities.copies?.max ?? null;
  if (copiesMax && copiesMax > 1) {
    chips.push({
      icon: "copies",
      key: "copies",
      label: `多份打印 · 最多 ${copiesMax}`,
      tone: "muted",
    });
  }

  if (capabilities.supports_page_ranges === true) {
    chips.push({
      icon: "page-ranges",
      key: "page-ranges",
      label: "页码范围",
      tone: "muted",
    });
  }

  const orientations = capabilities.orientations_supported.filter(Boolean);
  if (orientations.length > 1) {
    chips.push({
      icon: "orientation",
      key: "orientation",
      label: "方向可选",
      tone: "muted",
    });
  }

  const scalings = capabilities.scalings_supported.filter(Boolean);
  if (scalings.length > 1) {
    chips.push({
      icon: "scaling",
      key: "scaling",
      label: "缩放可选",
      tone: "muted",
    });
  }

  return chips;
}

const baseMediaLabels: Record<string, string> = {
  iso_a5_148x210mm: "A5",
  iso_a4_210x297mm: "A4",
  iso_a3_297x420mm: "A3",
  iso_a2_420x594mm: "A2",
  iso_a1_594x841mm: "A1",
  "na_letter_8.5x11in": "Letter",
  "na_legal_8.5x14in": "Legal",
  "na_number-10_4.125x9.5in": "No.10 Envelope",
  iso_dl_110x220mm: "DL Envelope",
  iso_c5_162x229mm: "C5 Envelope",
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
  "two-sided-long-edge": "双面长边翻页",
  "two-sided-short-edge": "双面短边翻页",
};

function formatMediaLabel(value: string) {
  if (baseMediaLabels[value]) return baseMediaLabels[value];
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

function printerSourceLabel(source: PrinterSource) {
  switch (source) {
    case "manual":
      return "手动添加";
    case "cups_import":
      return "CUPS 导入";
    case "mdns":
      return "网络发现";
  }
}

function getErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error && error.message.trim() ? error.message : fallback;
}
