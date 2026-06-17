import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ChevronDownIcon,
  CodeIcon,
  DatabaseIcon,
  FileTextIcon,
  FolderIcon,
  LayoutTemplateIcon,
  Loader2Icon,
  PencilIcon,
  PlusIcon,
  PrinterIcon,
  SaveIcon,
  SearchIcon,
  Trash2Icon,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { requestJson } from "../api";
import type { DeepprintController } from "../controller";
import { createTemplateWorkspaceQueryOptions } from "../queries";
import type { TemplateGroup, TemplateGroupResponse, TemplateRecord, TemplateResponse } from "../types";

const PdfPreview = lazy(() =>
  import("@/components/pdf-preview").then((module) => ({
    default: module.PdfPreview,
  })),
);
const ThemedCodeEditor = lazy(() =>
  import("@/components/themed-code-editor").then((module) => ({
    default: module.ThemedCodeEditor,
  })),
);

type TemplatesPageProps = {
  controller: DeepprintController;
  onNavigatePrint: () => void;
};

type DialogMode =
  | { kind: "group-create" }
  | { kind: "group-edit"; group: TemplateGroup }
  | { kind: "template-create"; groupId?: string }
  | { kind: "template-edit"; template: TemplateRecord }
  | null;

type DeleteTarget =
  | { kind: "group"; group: TemplateGroup }
  | { kind: "template"; template: TemplateRecord }
  | null;

const DEFAULT_TEMPLATE_CODE =
  "#set page(width: 80mm, height: auto)\n#set text(size: 10pt)\n\n= #data.title\n\n#data.message";
const DEFAULT_TEMPLATE_DATA = JSON.stringify(
  { title: "DeepPrint", message: "Hello receipt" },
  null,
  2,
);
const PREVIEW_DEBOUNCE_MS = 180;

export function TemplatesPage({ controller, onNavigatePrint }: TemplatesPageProps) {
  const { actions, ui, writes } = controller;
  const queryClient = useQueryClient();
  const [groups, setGroups] = useState<TemplateGroup[]>([]);
  const [workspaceRefreshing, setWorkspaceRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({});
  const [activeTemplateId, setActiveTemplateId] = useState("");
  const [editorTemplateId, setEditorTemplateId] = useState("");
  const [codeValue, setCodeValue] = useState("");
  const [dataValue, setDataValue] = useState("");
  const [saving, setSaving] = useState(false);
  const [dialog, setDialog] = useState<DialogMode>(null);
  const [dialogName, setDialogName] = useState("");
  const [dialogDescription, setDialogDescription] = useState("");
  const [dialogGroupId, setDialogGroupId] = useState("");
  const [dialogSubmitting, setDialogSubmitting] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<DeleteTarget>(null);
  const [deleteSubmitting, setDeleteSubmitting] = useState(false);
  const [activeTab, setActiveTab] = useState("code");
  const [switcherOpen, setSwitcherOpen] = useState(false);
  const switcherRef = useRef<HTMLDivElement | null>(null);
  const previewTypstRef = useRef(actions.onPreviewTypst);
  const previewRequestKeyRef = useRef("");

  const workspaceQueryOptions = useMemo(
    () => createTemplateWorkspaceQueryOptions(ui.baseUrl, ui.requestTimeouts.writes),
    [ui.baseUrl, ui.requestTimeouts.writes],
  );
  const workspaceQuery = useQuery(workspaceQueryOptions);
  const loading = workspaceQuery.isLoading || workspaceRefreshing;

  const allTemplates = useMemo(() => groups.flatMap((group) => group.templates), [groups]);
  const activeTemplate = useMemo(
    () =>
      allTemplates.find((template) => template.id === activeTemplateId) ??
      allTemplates[0] ??
      null,
    [activeTemplateId, allTemplates],
  );
  const activeGroup = useMemo(
    () => groups.find((group) => group.id === activeTemplate?.group_id) ?? null,
    [activeTemplate?.group_id, groups],
  );
  const hasUnsavedChanges = Boolean(
    activeTemplate &&
      (codeValue !== activeTemplate.typst_code || dataValue !== activeTemplate.sample_data),
  );

  const filteredGroups = useMemo(() => {
    const keyword = search.trim().toLowerCase();
    if (!keyword) return groups;
    return groups
      .map((group) => {
        const groupMatches = group.name.toLowerCase().includes(keyword);
        if (groupMatches) return group;
        return {
          ...group,
          templates: group.templates.filter(
            (template) =>
              template.name.toLowerCase().includes(keyword) ||
              template.description.toLowerCase().includes(keyword),
          ),
        };
      })
      .filter((group) => group.templates.length > 0 || group.name.toLowerCase().includes(keyword));
  }, [groups, search]);

  const applyWorkspace = useCallback((workspaceGroups: TemplateGroup[], preferredTemplateId?: string) => {
    setGroups(workspaceGroups);
    setExpandedGroups((current) => {
      const next = { ...current };
      for (const group of workspaceGroups) {
        if (!(group.id in next)) next[group.id] = true;
      }
      return next;
    });
    const nextTemplates = workspaceGroups.flatMap((group) => group.templates);
    setActiveTemplateId((current) => {
      if (preferredTemplateId && nextTemplates.some((item) => item.id === preferredTemplateId)) {
        return preferredTemplateId;
      }
      if (current && nextTemplates.some((item) => item.id === current)) {
        return current;
      }
      return nextTemplates[0]?.id ?? "";
    });
  }, []);

  const loadWorkspace = useCallback(
    async (preferredTemplateId?: string) => {
      setWorkspaceRefreshing(true);
      setError(null);
      try {
        const workspace = await queryClient.fetchQuery({
          ...workspaceQueryOptions,
          staleTime: 0,
        });
        applyWorkspace(workspace.groups, preferredTemplateId);
      } catch (err) {
        const message = err instanceof Error ? err.message : "加载模板失败";
        setError(message);
      } finally {
        setWorkspaceRefreshing(false);
      }
    },
    [applyWorkspace, queryClient, workspaceQueryOptions],
  );

  useEffect(() => {
    if (workspaceQuery.data) {
      setError(null);
      applyWorkspace(workspaceQuery.data.groups);
    }
  }, [applyWorkspace, workspaceQuery.data]);

  useEffect(() => {
    if (workspaceQuery.error) {
      const message = workspaceQuery.error instanceof Error ? workspaceQuery.error.message : "加载模板失败";
      setError(message);
    }
  }, [workspaceQuery.error]);

  useEffect(() => {
    previewTypstRef.current = actions.onPreviewTypst;
  }, [actions.onPreviewTypst]);

  useEffect(() => {
    if (!activeTemplate) {
      setEditorTemplateId("");
      setCodeValue("");
      setDataValue("");
      previewRequestKeyRef.current = "";
      return;
    }
    setEditorTemplateId(activeTemplate.id);
    setCodeValue(activeTemplate.typst_code);
    setDataValue(activeTemplate.sample_data);
    setActiveTab("code");
  }, [activeTemplate]);

  useEffect(() => {
    if (!activeTemplate || editorTemplateId !== activeTemplate.id) return;
    const previewKey = JSON.stringify([activeTemplate.id, codeValue, dataValue]);
    if (previewRequestKeyRef.current === previewKey) return;

    const timeout = window.setTimeout(() => {
      previewRequestKeyRef.current = previewKey;
      void previewTypstRef.current({
        templateContent: codeValue,
        dataJson: dataValue,
      });
    }, PREVIEW_DEBOUNCE_MS);
    return () => window.clearTimeout(timeout);
  }, [activeTemplate, codeValue, dataValue, editorTemplateId]);

  useEffect(() => {
    if (!switcherOpen) return;
    const closeOnOutsideClick = (event: MouseEvent | TouchEvent) => {
      if (switcherRef.current?.contains(event.target as Node)) return;
      setSwitcherOpen(false);
    };
    document.addEventListener("mousedown", closeOnOutsideClick);
    document.addEventListener("touchstart", closeOnOutsideClick);
    return () => {
      document.removeEventListener("mousedown", closeOnOutsideClick);
      document.removeEventListener("touchstart", closeOnOutsideClick);
    };
  }, [switcherOpen]);

  const openDialog = (next: DialogMode) => {
    setDialog(next);
    if (next?.kind === "group-edit") {
      setDialogName(next.group.name);
      setDialogDescription("");
      setDialogGroupId(next.group.id);
      return;
    }
    if (next?.kind === "template-edit") {
      setDialogName(next.template.name);
      setDialogDescription(next.template.description);
      setDialogGroupId(next.template.group_id);
      return;
    }
    if (next?.kind === "template-create") {
      setDialogName("");
      setDialogDescription("");
      setDialogGroupId(next.groupId ?? groups[0]?.id ?? "");
      return;
    }
    setDialogName("");
    setDialogDescription("");
    setDialogGroupId("");
  };

  const submitDialog = async () => {
    if (!dialog) return;
    setDialogSubmitting(true);
    try {
      if (dialog.kind === "group-create") {
        const result = await requestJson<TemplateGroupResponse>(
          ui.baseUrl,
          "/v1/templates/groups/create",
          {
            method: "POST",
            body: JSON.stringify({ name: dialogName }),
            timeoutMs: ui.requestTimeouts.writes,
          },
        );
        await loadWorkspace(result.group.templates[0]?.id);
      }

      if (dialog.kind === "group-edit") {
        await requestJson<TemplateGroupResponse>(
          ui.baseUrl,
          `/v1/templates/groups/${encodeURIComponent(dialog.group.id)}/update`,
          {
            method: "POST",
            body: JSON.stringify({ name: dialogName }),
            timeoutMs: ui.requestTimeouts.writes,
          },
        );
        await loadWorkspace(activeTemplateId);
      }

      if (dialog.kind === "template-create") {
        const result = await requestJson<TemplateResponse>(
          ui.baseUrl,
          "/v1/templates/create",
          {
            method: "POST",
            body: JSON.stringify({
              group_id: dialogGroupId,
              name: dialogName,
              description: dialogDescription,
              output_name: `${dialogName || "template"}.pdf`,
              typst_code: DEFAULT_TEMPLATE_CODE,
              sample_data: DEFAULT_TEMPLATE_DATA,
            }),
            timeoutMs: ui.requestTimeouts.writes,
          },
        );
        await loadWorkspace(result.template.id);
      }

      if (dialog.kind === "template-edit") {
        const result = await requestJson<TemplateResponse>(
          ui.baseUrl,
          `/v1/templates/${encodeURIComponent(dialog.template.id)}/update`,
          {
            method: "POST",
            body: JSON.stringify({
              group_id: dialogGroupId,
              name: dialogName,
              description: dialogDescription,
              output_name: dialog.template.output_name,
              typst_code: dialog.template.id === activeTemplate?.id ? codeValue : dialog.template.typst_code,
              sample_data: dialog.template.id === activeTemplate?.id ? dataValue : dialog.template.sample_data,
            }),
            timeoutMs: ui.requestTimeouts.writes,
          },
        );
        await loadWorkspace(result.template.id);
      }

      setDialog(null);
      ui.setNotice({ kind: "ok", message: "模板信息已保存" });
    } catch (err) {
      const message = err instanceof Error ? err.message : "保存失败";
      ui.setNotice({ kind: "error", message });
    } finally {
      setDialogSubmitting(false);
    }
  };

  const saveActiveTemplate = async () => {
    if (!activeTemplate) return;
    setSaving(true);
    try {
      const result = await requestJson<TemplateResponse>(
        ui.baseUrl,
        `/v1/templates/${encodeURIComponent(activeTemplate.id)}/update`,
        {
          method: "POST",
          body: JSON.stringify({
            group_id: activeTemplate.group_id,
            name: activeTemplate.name,
            description: activeTemplate.description,
            output_name: activeTemplate.output_name,
            typst_code: codeValue,
            sample_data: dataValue,
          }),
          timeoutMs: ui.requestTimeouts.writes,
        },
      );
      await loadWorkspace(result.template.id);
      ui.setNotice({ kind: "ok", message: "模板已保存" });
    } catch (err) {
      const message = err instanceof Error ? err.message : "保存模板失败";
      ui.setNotice({ kind: "error", message });
    } finally {
      setSaving(false);
    }
  };

  const deleteGroup = async (group: TemplateGroup) => {
    try {
      await requestJson<{ deleted: boolean }>(
        ui.baseUrl,
        `/v1/templates/groups/${encodeURIComponent(group.id)}/delete`,
        { method: "POST", body: JSON.stringify({}), timeoutMs: ui.requestTimeouts.writes },
      );
      await loadWorkspace(activeTemplateId);
      ui.setNotice({ kind: "ok", message: "分组已删除" });
    } catch (err) {
      const message = err instanceof Error ? err.message : "删除分组失败";
      ui.setNotice({ kind: "error", message });
    }
  };

  const deleteTemplate = async (template: TemplateRecord) => {
    try {
      await requestJson<{ deleted: boolean }>(
        ui.baseUrl,
        `/v1/templates/${encodeURIComponent(template.id)}/delete`,
        { method: "POST", body: JSON.stringify({}), timeoutMs: ui.requestTimeouts.writes },
      );
      await loadWorkspace("");
      ui.setNotice({ kind: "ok", message: "模板已删除" });
    } catch (err) {
      const message = err instanceof Error ? err.message : "删除模板失败";
      ui.setNotice({ kind: "error", message });
    }
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleteSubmitting(true);
    try {
      if (deleteTarget.kind === "group") {
        await deleteGroup(deleteTarget.group);
      } else {
        await deleteTemplate(deleteTarget.template);
      }
      setDeleteTarget(null);
    } finally {
      setDeleteSubmitting(false);
    }
  };

  const chooseTemplate = (templateId: string) => {
    setActiveTemplateId(templateId);
    setSwitcherOpen(false);
    setSearch("");
  };

  const printActiveTemplate = () => {
    if (!activeTemplate) return;
    writes.setCreateTemplateContent(codeValue);
    writes.setCreateDataJson(dataValue);
    writes.setCreateRequestId(`req_${Date.now()}`);
    void actions.onPreviewTypst({
      templateContent: codeValue,
      dataJson: dataValue,
    });
    onNavigatePrint();
  };

  return (
    <main className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background">
      <header className="relative z-30 shrink-0 border-b bg-background/95 px-3 py-3 backdrop-blur sm:px-5">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div ref={switcherRef} className="relative min-w-0 flex-1">
            <div className="flex min-w-0 items-center">
              <button
                type="button"
                className={cn(
                  "group -ml-2 flex min-w-0 items-center gap-2.5 rounded-lg px-2 py-1.5 text-left transition-colors",
                  switcherOpen ? "bg-muted" : "hover:bg-muted",
                )}
                onClick={() => setSwitcherOpen((open) => !open)}
              >
                <span
                  className={cn(
                    "flex size-8 shrink-0 items-center justify-center rounded-md border border-transparent bg-muted text-muted-foreground transition-colors",
                    switcherOpen
                      ? "border-border bg-background text-primary shadow-sm"
                      : "group-hover:border-border group-hover:bg-background group-hover:text-foreground group-hover:shadow-sm",
                  )}
                >
                  <LayoutTemplateIcon className="size-4" />
                </span>
                <span className="flex min-w-0 flex-col justify-center">
                  <span className="mb-0.5 flex items-center gap-1.5 text-[11px] font-medium leading-tight text-muted-foreground">
                    <span className="truncate max-w-[8rem]">{activeGroup?.name ?? "模板管理"}</span>
                    <span>/</span>
                  </span>
                  <span className="flex min-w-0 items-center gap-1.5 text-sm font-semibold leading-tight">
                    <span className="truncate max-w-[14rem]">{activeTemplate?.name ?? "选择模板"}</span>
                    <ChevronDownIcon
                      className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform", switcherOpen && "rotate-180")}
                    />
                  </span>
                </span>
              </button>
            </div>

            {switcherOpen ? (
              <div className="absolute left-0 top-[calc(100%+0.625rem)] z-50 flex max-h-[min(72vh,34rem)] w-[min(calc(100vw-1.5rem),25rem)] flex-col overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-xl ring-1 ring-foreground/5 animate-in fade-in-0 zoom-in-95 sm:w-[25rem]">
                <div className="border-b p-2">
                  <div className="relative">
                    <SearchIcon className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      autoFocus
                      value={search}
                      onChange={(event) => setSearch(event.target.value)}
                      placeholder="搜索模板或分组..."
                      className="h-9 pl-9"
                    />
                  </div>
                </div>

                <div className="min-h-0 flex-1 overflow-y-auto p-2">
                  {loading && groups.length === 0 ? (
                    <SwitcherEmpty>正在加载模板...</SwitcherEmpty>
                  ) : error ? (
                    <SwitcherEmpty tone="destructive">{error}</SwitcherEmpty>
                  ) : filteredGroups.length === 0 ? (
                    <SwitcherEmpty>没有匹配的模板。</SwitcherEmpty>
                  ) : (
                    <div className="space-y-3">
                      {filteredGroups.map((group) => {
                        const expanded = search.trim() ? true : expandedGroups[group.id] ?? true;
                        return (
                          <div key={group.id} className="space-y-1">
                          <div className="group flex items-center gap-1 rounded-lg px-1 py-1 text-muted-foreground transition-colors hover:bg-muted/70 hover:text-foreground">
                            <button
                              type="button"
                              className="flex min-w-0 flex-1 items-center gap-2 rounded-md px-1 py-1 text-left text-xs font-semibold uppercase tracking-wide"
                              onClick={() =>
                                setExpandedGroups((current) => ({
                                  ...current,
                                  [group.id]: !(current[group.id] ?? true),
                                }))
                              }
                            >
                              <ChevronDownIcon
                                className={cn("size-3.5 shrink-0 transition-transform", expanded ? "" : "-rotate-90")}
                              />
                              <FolderIcon className="size-4 shrink-0 stroke-[2.25]" />
                              <span className="truncate">{group.name}</span>
                              <span className="rounded-full bg-background/80 px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground ring-1 ring-border/60 group-hover:text-foreground">
                                {group.templates.length}
                              </span>
                            </button>
                            <IconAction
                              label="新建模板"
                              icon={<PlusIcon />}
                              onClick={() => {
                                setSwitcherOpen(false);
                                openDialog({ kind: "template-create", groupId: group.id });
                              }}
                            />
                            <IconAction
                              label="编辑分组"
                              icon={<PencilIcon />}
                              onClick={() => {
                                setSwitcherOpen(false);
                                openDialog({ kind: "group-edit", group });
                              }}
                            />
                            <IconAction
                              danger
                              label="删除分组"
                              icon={<Trash2Icon />}
                              onClick={() => {
                                setSwitcherOpen(false);
                                setDeleteTarget({ kind: "group", group });
                              }}
                            />
                          </div>
                          {expanded ? (
                            <div className="space-y-1">
                              {group.templates.map((template) => {
                                const active = template.id === activeTemplate?.id;
                                return (
                                  <TemplateSwitcherItem
                                    key={template.id}
                                    active={active}
                                    template={template}
                                    onChoose={() => chooseTemplate(template.id)}
                                    onDelete={() => {
                                      setSwitcherOpen(false);
                                      setDeleteTarget({ kind: "template", template });
                                    }}
                                    onEdit={() => {
                                      setSwitcherOpen(false);
                                      openDialog({ kind: "template-edit", template });
                                    }}
                                  />
                                );
                              })}
                            </div>
                          ) : null}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                <div className="border-t bg-muted/35 p-2">
                  <Button
                    type="button"
                    variant="ghost"
                    className="w-full justify-center"
                    onClick={() => {
                      setSwitcherOpen(false);
                      openDialog({ kind: "group-create" });
                    }}
                  >
                    <PlusIcon data-icon="inline-start" />
                    新建分组
                  </Button>
                </div>
              </div>
            ) : null}
          </div>

          <div className="flex shrink-0 items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="icon"
              disabled={!activeTemplate}
              title="编辑模板信息"
              onClick={() => activeTemplate && openDialog({ kind: "template-edit", template: activeTemplate })}
            >
              <PencilIcon className="size-4" />
            </Button>
            <Button type="button" variant="outline" disabled={!activeTemplate} onClick={printActiveTemplate}>
              <PrinterIcon data-icon="inline-start" />
              <span className="hidden sm:inline">打印</span>
            </Button>
            <Button type="button" disabled={!activeTemplate || saving} onClick={() => void saveActiveTemplate()}>
              {saving ? <Loader2Icon data-icon="inline-start" className="animate-spin" /> : <SaveIcon data-icon="inline-start" />}
              <span>{hasUnsavedChanges ? "保存更改" : "保存"}</span>
            </Button>
          </div>
        </div>
      </header>

      <div className="relative z-0 min-h-0 flex-1 overflow-y-auto bg-muted/30 p-3 sm:p-4 lg:overflow-hidden lg:p-6">
        {activeTemplate ? (
          <div className="grid overflow-hidden rounded-xl border bg-card shadow-sm lg:h-full lg:min-h-0 lg:grid-cols-[minmax(0,1.05fr)_minmax(360px,0.95fr)]">
            <section className="flex min-h-[560px] min-w-0 flex-col overflow-hidden border-b lg:min-h-0 lg:border-b-0 lg:border-r">
              <div className="flex flex-col gap-3 border-b px-4 py-3 sm:flex-row sm:items-center sm:justify-between sm:px-5">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
                    <h2 className="truncate text-sm font-semibold">{activeTemplate.name}</h2>
                  </div>
                  <p className="mt-1 truncate text-xs text-muted-foreground">
                    {activeTemplate.description || activeTemplate.output_name || "Typst 模板"}
                  </p>
                </div>
                <Tabs value={activeTab} onValueChange={setActiveTab}>
                  <TabsList className="w-full sm:w-auto">
                    <TabsTrigger value="code" className="flex-1 sm:flex-none">
                      <CodeIcon data-icon="inline-start" />
                      Typst 代码
                    </TabsTrigger>
                    <TabsTrigger value="data" className="flex-1 sm:flex-none">
                      <DatabaseIcon data-icon="inline-start" />
                      JSON 数据
                    </TabsTrigger>
                  </TabsList>
                </Tabs>
              </div>

              <div className="min-h-0 flex-1 bg-background">
                <Suspense fallback={<EditorLoadingState />}>
                  {activeTab === "code" ? (
                    <ThemedCodeEditor
                      className="rounded-none ring-0 focus-within:ring-0"
                      fillHeight
                      language="typst"
                      onChange={setCodeValue}
                      placeholder="输入 Typst 模板代码..."
                      value={codeValue}
                    />
                  ) : (
                    <ThemedCodeEditor
                      className="rounded-none ring-0 focus-within:ring-0"
                      fillHeight
                      language="json"
                      placeholder="输入模板数据 JSON..."
                      value={dataValue}
                      onChange={setDataValue}
                    />
                  )}
                </Suspense>
              </div>

              <div className="flex flex-wrap items-center justify-between gap-2 border-t px-4 py-3 text-xs text-muted-foreground sm:px-5">
                <span>编辑后会自动刷新右侧预览。</span>
                <span className={cn("font-medium", hasUnsavedChanges ? "text-amber-600" : "text-muted-foreground")}>
                  {hasUnsavedChanges ? "有未保存更改" : "当前内容已保存"}
                </span>
              </div>
            </section>

            <section className="flex min-h-[560px] min-w-0 flex-col overflow-hidden lg:min-h-0">
              <div className="flex items-center justify-between gap-3 border-b px-4 py-3 sm:px-5">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="size-2 rounded-full bg-emerald-500" />
                    <h2 className="text-sm font-semibold">实时预览</h2>
                  </div>
                  <p className="mt-1 truncate text-xs text-muted-foreground">
                    {activeTemplate.output_name || "预览 PDF"}
                  </p>
                </div>
                <Badge variant="outline">PDF</Badge>
              </div>

              <div className="min-h-0 flex-1 bg-muted/30">
                <Suspense fallback={<PreviewLoadingState />}>
                  <PdfPreview
                    className="h-full min-h-[460px] lg:min-h-0"
                    emptyMessage="编辑模板代码或示例数据后会自动生成预览。"
                    loading={writes.previewLoading}
                    source={writes.previewPdfUrl}
                  />
                </Suspense>
              </div>

              {writes.previewError ? (
                <div className="border-t px-4 py-3 sm:px-5">
                  <Badge variant="destructive" className="h-auto whitespace-normal text-left">
                    {writes.previewError}
                  </Badge>
                </div>
              ) : (
                <div className="flex items-center justify-between border-t px-4 py-3 text-xs text-muted-foreground sm:px-5">
                  <span>Typst 渲染结果</span>
                  <span>{activeTemplate.output_name || "template.pdf"}</span>
                </div>
              )}
            </section>
          </div>
        ) : (
          <Card className="mx-auto max-w-2xl">
            <CardHeader className="text-center">
              <CardTitle>{loading ? "正在加载模板" : "还没有可编辑的模板"}</CardTitle>
              <CardDescription>
                {error
                  ? error
                  : loading
                    ? "模板工作台马上就绪。"
                    : "先创建分组，再创建模板，就可以在这里编辑 Typst 和示例数据。"}
              </CardDescription>
            </CardHeader>
            <CardContent className="flex flex-wrap justify-center gap-2">
              <Button type="button" variant="outline" onClick={() => openDialog({ kind: "group-create" })}>
                <PlusIcon data-icon="inline-start" />
                新建分组
              </Button>
              {groups.length > 0 ? (
                <Button type="button" onClick={() => openDialog({ kind: "template-create", groupId: groups[0]?.id })}>
                  <FileTextIcon data-icon="inline-start" />
                  新建模板
                </Button>
              ) : null}
            </CardContent>
          </Card>
        )}
      </div>

      {dialog ? (
        <div className="pointer-events-none fixed inset-0 z-50 flex items-center justify-center p-4">
          <Card className="pointer-events-auto max-h-[calc(100vh-5rem)] w-full max-w-lg overflow-y-auto shadow-xl sm:w-[32rem]">
            <CardHeader>
              <CardTitle>{dialogTitle(dialog)}</CardTitle>
              <CardDescription>{dialogDescriptionText(dialog)}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="dialog-name">名称</Label>
                <Input
                  id="dialog-name"
                  value={dialogName}
                  onChange={(event) => setDialogName(event.target.value)}
                  placeholder="请输入名称"
                />
              </div>
              {dialog.kind === "template-create" || dialog.kind === "template-edit" ? (
                <>
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="dialog-description">描述</Label>
                    <Input
                      id="dialog-description"
                      value={dialogDescription}
                      onChange={(event) => setDialogDescription(event.target.value)}
                      placeholder="可选"
                    />
                  </div>
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="dialog-group">所属分组</Label>
                    <label className="flex h-9 items-center rounded-lg border bg-background px-3 text-sm">
                      <select
                        id="dialog-group"
                        value={dialogGroupId}
                        onChange={(event) => setDialogGroupId(event.target.value)}
                        className="w-full appearance-none bg-transparent outline-none"
                      >
                        {groups.map((group) => (
                          <option key={group.id} value={group.id}>
                            {group.name}
                          </option>
                        ))}
                      </select>
                      <ChevronDownIcon className="size-4 shrink-0 text-muted-foreground" />
                    </label>
                  </div>
                </>
              ) : null}
              <div className="flex justify-end gap-2 pt-2">
                <Button type="button" variant="outline" onClick={() => setDialog(null)}>
                  取消
                </Button>
                <Button
                  type="button"
                  disabled={
                    dialogSubmitting ||
                    !dialogName.trim() ||
                    ((dialog.kind === "template-create" || dialog.kind === "template-edit") && !dialogGroupId)
                  }
                  onClick={() => void submitDialog()}
                >
                  保存
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      ) : null}

      {deleteTarget ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 p-4 backdrop-blur-sm">
          <Card className="w-full max-w-md shadow-xl">
            <CardHeader>
              <CardTitle>{deleteTarget.kind === "group" ? "删除分组" : "删除模板"}</CardTitle>
              <CardDescription>
                {deleteTarget.kind === "group"
                  ? `确认删除分组“${deleteTarget.group.name}”？如果分组下还有模板，系统会阻止删除。`
                  : `确认删除模板“${deleteTarget.template.name}”？此操作不可撤销。`}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex justify-end gap-2">
                <Button type="button" variant="outline" disabled={deleteSubmitting} onClick={() => setDeleteTarget(null)}>
                  取消
                </Button>
                <Button type="button" variant="destructive" disabled={deleteSubmitting} onClick={() => void confirmDelete()}>
                  删除
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      ) : null}
    </main>
  );
}

function TemplateSwitcherItem({
  active,
  onChoose,
  onDelete,
  onEdit,
  template,
}: {
  active: boolean;
  onChoose: () => void;
  onDelete: () => void;
  onEdit: () => void;
  template: TemplateRecord;
}) {
  return (
    <div
      className={cn(
        "group/item flex items-center gap-1 rounded-lg px-1 transition-colors",
        active ? "bg-primary/10 text-primary" : "text-popover-foreground hover:bg-muted",
      )}
    >
      <button type="button" className="flex min-w-0 flex-1 items-center gap-2 px-1.5 py-2 text-left" onClick={onChoose}>
        <FileTextIcon className={cn("size-4 shrink-0", active ? "text-primary" : "text-muted-foreground")} />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm font-medium">{template.name}</span>
          {template.description ? (
            <span className={cn("block truncate text-[11px]", active ? "text-primary/70" : "text-muted-foreground")}>
              {template.description}
            </span>
          ) : null}
        </span>
      </button>
      <IconAction label="编辑模板" icon={<PencilIcon />} onClick={onEdit} />
      <IconAction danger label="删除模板" icon={<Trash2Icon />} onClick={onDelete} />
    </div>
  );
}

function IconAction({
  danger,
  icon,
  label,
  onClick,
}: {
  danger?: boolean;
  icon: ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={cn(
        "flex size-7 shrink-0 items-center justify-center rounded-lg opacity-100 transition-colors sm:opacity-0 sm:group-hover:opacity-100 sm:group-hover/item:opacity-100",
        danger
          ? "text-destructive hover:bg-destructive/10"
          : "text-muted-foreground hover:bg-background hover:text-foreground",
      )}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      <span className="[&_svg]:size-3.5">{icon}</span>
    </button>
  );
}

function SwitcherEmpty({ children, tone }: { children: ReactNode; tone?: "destructive" }) {
  return (
    <div
      className={cn(
        "rounded-xl border bg-muted/30 p-6 text-center text-sm text-muted-foreground",
        tone === "destructive" && "text-destructive",
      )}
    >
      {children}
    </div>
  );
}

function dialogTitle(dialog: Exclude<DialogMode, null>) {
  switch (dialog.kind) {
    case "group-create":
      return "新建分组";
    case "group-edit":
      return "编辑分组";
    case "template-create":
      return "新建模板";
    case "template-edit":
      return "编辑模板信息";
  }
}

function dialogDescriptionText(dialog: Exclude<DialogMode, null>) {
  switch (dialog.kind) {
    case "group-create":
      return "创建一个模板分组。";
    case "group-edit":
      return "修改模板分组名称。";
    case "template-create":
      return "创建模板后可以编辑 Typst 代码和示例数据。";
    case "template-edit":
      return "修改模板名称、描述和所属分组。";
  }
}

function EditorLoadingState() {
  return (
    <div className="flex h-full min-h-[420px] items-center justify-center bg-background text-sm text-muted-foreground">
      正在加载编辑器...
    </div>
  );
}

function PreviewLoadingState() {
  return (
    <div className="flex h-full min-h-[460px] items-center justify-center bg-muted/20 text-sm text-muted-foreground">
      正在加载预览器...
    </div>
  );
}
