import { useMemo, useRef, useState } from "react";
import {
  RefreshCwIcon,
  SearchIcon,
  Trash2Icon,
  TypeIcon,
  UploadIcon,
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
import type { DeepprintController } from "@/features/deepprint/controller";
import type { TypstFontInfo } from "@/features/deepprint/types";
import { formatBytes } from "@/features/deepprint/utils";
import { cn } from "@/lib/utils";

export function FontsSection({ controller }: { controller: DeepprintController }) {
  const { actions, typstFonts } = controller;
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLowerCase();
  const fonts = useMemo(
    () =>
      typstFonts.fonts
        .slice()
        .sort((left, right) => left.file_name.localeCompare(right.file_name)),
    [typstFonts.fonts],
  );
  const filteredFonts = useMemo(
    () => filterFonts(fonts, normalizedQuery),
    [fonts, normalizedQuery],
  );

  const onFontFileSelected = async (file: File | undefined) => {
    if (!file) return;
    await actions.onInstallTypstFont(file);
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  return (
    <div className="flex animate-in flex-col gap-4 duration-300 fade-in slide-in-from-bottom-2">
      <Card size="sm">
        <CardHeader className="gap-4">
          <div className="space-y-1">
            <CardTitle className="flex items-center gap-2">
              <TypeIcon className="size-4 text-amber-600" />
              Typst 字体管理
            </CardTitle>
            <CardDescription>
              统一管理当前实例中 Typst 可直接使用的字体。默认字体会在初始化时写入统一字体目录，后续上传字体也在这里一起管理。
            </CardDescription>
          </div>
          <CardAction className="flex w-full flex-wrap gap-2 sm:w-auto">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={typstFonts.loading}
              onClick={() => void actions.loadTypstFonts(false)}
            >
              <RefreshCwIcon
                data-icon="inline-start"
                className={cn(typstFonts.loading ? "animate-spin" : "")}
              />
              刷新
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={typstFonts.installing}
              onClick={() => fileInputRef.current?.click()}
            >
              <UploadIcon data-icon="inline-start" />
              {typstFonts.installing ? "上传中..." : "上传字体"}
            </Button>
          </CardAction>
        </CardHeader>

        <CardContent className="space-y-6">
          <InputFile
            fileInputRef={fileInputRef}
            disabled={typstFonts.installing}
            onSelect={(file) => void onFontFileSelected(file)}
          />

          {typstFonts.error ? (
            <Badge variant="destructive" className="h-auto whitespace-normal">
              {typstFonts.error}
            </Badge>
          ) : null}

          <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="space-y-1">
              <div className="text-sm font-medium text-foreground">统一字体列表</div>
              <div className="text-xs leading-5 text-muted-foreground">
                当前环境中可供 Typst 直接使用的字体都会在这里展示与管理。
              </div>
            </div>

            <div className="relative w-full lg:max-w-xs">
              <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.currentTarget.value)}
                placeholder="检索字体"
                className="pl-8"
              />
            </div>
          </div>

          <FontListPanel
            description="支持按文件名或扩展名检索，删除后会立即从统一字体目录中移除。"
            emptyTitle={normalizedQuery ? "没有匹配的字体" : "还没有字体"}
            emptyDescription={
              normalizedQuery
                ? "可以试试搜索文件名、字体名或扩展名。"
                : "支持上传 `.ttf`、`.otf`、`.ttc`、`.otc` 字体文件，上传后即可在模板中引用。"
            }
            fonts={filteredFonts}
            query={normalizedQuery}
            totalCount={fonts.length}
            deletingName={typstFonts.deletingName}
            onDelete={(font) => void actions.onDeleteTypstFont(font)}
          />
        </CardContent>
      </Card>
    </div>
  );
}

function InputFile({
  disabled,
  fileInputRef,
  onSelect,
}: {
  disabled: boolean;
  fileInputRef: React.RefObject<HTMLInputElement | null>;
  onSelect: (file: File | undefined) => void;
}) {
  return (
    <input
      ref={fileInputRef}
      type="file"
      accept=".ttf,.otf,.ttc,.otc"
      disabled={disabled}
      onChange={(event) => onSelect(event.currentTarget.files?.[0])}
      className="hidden"
    />
  );
}

function FontListPanel({
  deletingName,
  description,
  emptyDescription,
  emptyTitle,
  fonts,
  onDelete,
  query,
  totalCount,
}: {
  deletingName: string | null;
  description: string;
  emptyDescription: string;
  emptyTitle: string;
  fonts: TypstFontInfo[];
  onDelete: (font: TypstFontInfo) => void;
  query: string;
  totalCount: number;
}) {
  return (
    <div className="overflow-hidden rounded-2xl border bg-card/80">
      <div className="flex flex-col gap-2 border-b bg-muted/10 px-4 py-3 sm:flex-row sm:items-center sm:justify-between sm:px-5">
        <p className="text-xs leading-5 text-muted-foreground">{description}</p>
        <div className="flex items-center gap-2">
          <Badge variant="outline">
            {query ? `已显示 ${fonts.length} / ${totalCount}` : `共 ${totalCount} 条`}
          </Badge>
          {query ? <Badge variant="secondary">已筛选</Badge> : null}
        </div>
      </div>

      {!fonts.length ? (
        <div className="flex min-h-56 flex-col items-center justify-center px-4 py-10 text-center">
          <TypeIcon className="mb-3 size-10 text-muted-foreground/35" />
          <div className="text-sm font-medium text-foreground">{emptyTitle}</div>
          <div className="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
            {emptyDescription}
          </div>
          {query ? <div className="mt-3 text-[11px] text-muted-foreground">当前已应用检索条件</div> : null}
        </div>
      ) : (
        <div className="divide-y">
          <div className="hidden grid-cols-[minmax(0,1.6fr)_auto_auto] items-center gap-4 bg-muted/20 px-5 py-2 text-[11px] font-medium tracking-wide text-muted-foreground uppercase md:grid">
            <div>字体</div>
            <div>大小</div>
            <div className="text-right">操作</div>
          </div>
          {fonts.map((font) => (
            <FontListRow
              key={buildFontKey(font)}
              font={font}
              deleting={deletingName === font.file_name}
              onDelete={() => onDelete(font)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function FontListRow({
  deleting,
  font,
  onDelete,
}: {
  deleting: boolean;
  font: TypstFontInfo;
  onDelete: () => void;
}) {
  return (
    <div className="px-4 py-4 transition-colors hover:bg-muted/10 sm:px-5">
      <div className="flex flex-col gap-4 md:grid md:grid-cols-[minmax(0,1.6fr)_auto_auto] md:items-center">
        <div className="flex min-w-0 items-start gap-3">
          <div className="mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-xl border border-amber-200 bg-amber-50 text-amber-700">
            <span className="font-serif text-base font-semibold leading-none">A</span>
          </div>
          <div className="min-w-0 space-y-2">
            <div className="flex flex-wrap items-center gap-2">
              <div className="truncate text-sm font-semibold text-foreground">
                {stripExtension(font.file_name)}
              </div>
              <FileExtensionBadge filename={font.file_name} />
              <Badge variant="outline" className="font-mono text-[11px] md:hidden">
                {formatBytes(font.size_bytes)}
              </Badge>
            </div>
            <div className="text-xs text-muted-foreground">
              文件名：<span className="font-mono">{font.file_name}</span>
            </div>
          </div>
        </div>

        <div className="hidden text-xs font-medium text-muted-foreground md:block md:text-right">
          {formatBytes(font.size_bytes)}
        </div>

        <div className="flex shrink-0 justify-end">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={deleting}
            onClick={onDelete}
          >
            <Trash2Icon data-icon="inline-start" />
            {deleting ? "删除中..." : "删除"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function FileExtensionBadge({ filename }: { filename: string }) {
  const ext = filename.split(".").pop()?.toLowerCase() ?? "";
  return (
    <Badge
      variant="outline"
      className={cn(
        "text-[11px] uppercase",
        ext === "ttf" && "border-sky-200 bg-sky-50 text-sky-700",
        ext === "otf" && "border-amber-200 bg-amber-50 text-amber-700",
        ext === "ttc" && "border-violet-200 bg-violet-50 text-violet-700",
        ext === "otc" && "border-emerald-200 bg-emerald-50 text-emerald-700",
        !["ttf", "otf", "ttc", "otc"].includes(ext) && "border-border bg-muted text-muted-foreground",
      )}
    >
      {ext || "font"}
    </Badge>
  );
}

function stripExtension(fileName: string) {
  return fileName.split(".").slice(0, -1).join(".") || fileName;
}

function buildFontKey(font: TypstFontInfo) {
  return `${font.file_name}:${font.size_bytes}`;
}

function filterFonts(fonts: TypstFontInfo[], query: string) {
  if (!query) return fonts;

  return fonts.filter((font) => {
    const ext = font.file_name.split(".").pop()?.toLowerCase() ?? "";
    const normalizedName = stripExtension(font.file_name).toLowerCase();
    const normalizedFileName = font.file_name.toLowerCase();
    return (
      normalizedName.includes(query) ||
      normalizedFileName.includes(query) ||
      ext.includes(query)
    );
  });
}
