import { useRef } from "react";
import {
  PackageIcon,
  RefreshCwIcon,
  Trash2Icon,
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
import type { TypstPackageInfo } from "@/features/deepprint/types";
import { cn } from "@/lib/utils";
import { useI18n } from "@/i18n";

export function TypstPackagesSection({ controller }: { controller: DeepprintController }) {
  const { t } = useI18n();
  const { actions, typstPackages } = controller;
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const previewPackages = typstPackages.packages.filter((pkg) => pkg.origin === "preview_cache");

  const onPackageFileSelected = async (file: File | undefined) => {
    if (!file) return;
    await actions.onInstallTypstPackage(file);
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
              <PackageIcon className="size-4 text-primary" />
              {t("settings.packages.title")}
            </CardTitle>
            <CardDescription>
              {t("settings.packages.summary", {
                total: typstPackages.packages.length,
                local: typstPackages.packages.length - previewPackages.length,
                preview: previewPackages.length,
              })}
            </CardDescription>
          </div>
          <CardAction className="flex w-full flex-wrap gap-2 sm:w-auto">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={typstPackages.loading}
              onClick={() => void actions.loadTypstPackages(false)}
            >
              <RefreshCwIcon
                data-icon="inline-start"
                className={cn(typstPackages.loading ? "animate-spin" : "")}
              />
              {t("common.refresh")}
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={typstPackages.installing}
              onClick={() => fileInputRef.current?.click()}
            >
              <UploadIcon data-icon="inline-start" />
              {typstPackages.installing ? t("settings.packages.uploading") : t("settings.packages.upload")}
            </Button>
          </CardAction>
        </CardHeader>

        <CardContent className="space-y-4">
          <Input
            ref={fileInputRef}
            type="file"
            accept=".zip,.tar,.gz,.tgz"
            disabled={typstPackages.installing}
            onChange={(event) => void onPackageFileSelected(event.currentTarget.files?.[0])}
            className="hidden"
          />

          <div className="flex flex-col gap-3 rounded-xl border bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="space-y-1">
              <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
                {t("settings.packages.installedList")}
              </div>
              <div className="text-sm text-foreground">
                {t("settings.packages.listDescription")}
              </div>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={typstPackages.clearingPreviewCache || previewPackages.length === 0}
              onClick={() => void actions.onClearTypstPreviewCache()}
            >
              <Trash2Icon data-icon="inline-start" />
              {typstPackages.clearingPreviewCache ? t("settings.packages.clearing") : t("settings.packages.clearPreviewCache")}
            </Button>
          </div>

          {typstPackages.error ? (
            <Badge variant="destructive" className="h-auto whitespace-normal">
              {typstPackages.error}
            </Badge>
          ) : null}

          <PackageList
            packages={typstPackages.packages}
            deletingKey={typstPackages.deletingKey}
            onDelete={(pkg) => void actions.onDeleteTypstPackage(pkg)}
          />
        </CardContent>
      </Card>
    </div>
  );
}

function PackageList({
  deletingKey,
  onDelete,
  packages,
}: {
  deletingKey: string | null;
  onDelete: (pkg: TypstPackageInfo) => void;
  packages: TypstPackageInfo[];
}) {
  const { t } = useI18n();

  if (!packages.length) {
    return (
      <div className="flex flex-col items-center rounded-xl border border-dashed bg-muted/20 px-4 py-10 text-center">
        <PackageIcon className="mb-3 size-10 text-muted-foreground/40" />
        <div className="text-sm font-medium text-foreground">{t("settings.packages.emptyTitle")}</div>
        <div className="mt-1 max-w-sm text-xs leading-5 text-muted-foreground">
          {t("settings.packages.emptyDescription")}
        </div>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3">
      {packages.map((pkg) => {
        const key = `${pkg.origin}:${pkg.namespace}/${pkg.name}:${pkg.version}`;
        const isPreview = pkg.origin === "preview_cache";
        return (
          <div
            key={key}
            className="group rounded-xl border bg-card px-4 py-4 transition-colors hover:border-primary/25"
          >
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div className="flex min-w-0 gap-3">
                <div
                  className={cn(
                    "mt-0.5 flex size-10 shrink-0 items-center justify-center rounded-xl border",
                    isPreview
                      ? "border-sky-200 bg-sky-50 text-sky-700"
                      : "border-fuchsia-200 bg-fuchsia-50 text-fuchsia-700",
                  )}
                >
                  <PackageIcon className="size-4" />
                </div>
                <div className="min-w-0 space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-sm font-semibold text-foreground">
                      <span className="text-muted-foreground">@</span>
                      {pkg.namespace} / {pkg.name}
                    </div>
                    <Badge variant="outline" className="font-mono text-[11px]">
                      v{pkg.version}
                    </Badge>
                    <OriginBadge origin={pkg.origin} />
                  </div>
                  <div className="rounded-lg bg-muted/40 px-2.5 py-2 font-mono text-[11px] text-muted-foreground">
                    {pkg.import_snippet}
                  </div>
                </div>
              </div>

              <div className="flex shrink-0 justify-end">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={deletingKey === key}
                  onClick={() => onDelete(pkg)}
                >
                  <Trash2Icon data-icon="inline-start" />
                  {deletingKey === key ? t("common.deleting") : t("common.delete")}
                </Button>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function OriginBadge({ origin }: { origin: TypstPackageInfo["origin"] }) {
  const isPreview = origin === "preview_cache";
  return (
    <Badge
      variant="outline"
      className={cn(
        "text-[11px]",
        isPreview
          ? "border-sky-200 bg-sky-50 text-sky-700"
          : "border-fuchsia-200 bg-fuchsia-50 text-fuchsia-700",
      )}
    >
      {isPreview ? "preview" : "local"}
    </Badge>
  );
}
