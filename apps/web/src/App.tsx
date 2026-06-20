import { Suspense, lazy, useState, type CSSProperties, type ReactNode } from "react"
import { AppSidebar } from "@/components/app-sidebar"
import { SiteHeader } from "@/components/site-header"
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar"
import { TooltipProvider } from "@/components/ui/tooltip"
import { SettingsProvider, useSettings } from "@/hooks/use-settings"
import { useDeepprintController } from "@/features/deepprint"
import type { AuthUser } from "@/features/auth/types"
import { useI18n } from "@/i18n"

const PrintPage = lazy(() =>
  import("@/features/deepprint/pages/PrintPage").then((module) => ({
    default: module.PrintPage,
  })),
)
const PrintHistoryPage = lazy(() =>
  import("@/features/deepprint/pages/PrintHistoryPage").then((module) => ({
    default: module.PrintHistoryPage,
  })),
)
const PrinterManagementPage = lazy(() =>
  import("@/features/deepprint/pages/PrinterManagementPage").then((module) => ({
    default: module.PrinterManagementPage,
  })),
)
const TemplatesPage = lazy(() =>
  import("@/features/deepprint/pages/TemplatesPage").then((module) => ({
    default: module.TemplatesPage,
  })),
)
const UsersPage = lazy(() =>
  import("@/features/auth/pages/UsersPage").then((module) => ({
    default: module.UsersPage,
  })),
)
const ApiKeysPage = lazy(() =>
  import("@/features/auth/pages/ApiKeysPage").then((module) => ({
    default: module.ApiKeysPage,
  })),
)
const SettingsOverlay = lazy(() =>
  import("@/components/settings-overlay").then((module) => ({
    default: module.SettingsOverlay,
  })),
)

export type AppPage =
  | "print"
  | "templates"
  | "printers"
  | "history"
  | "users"
  | "apiKeys"

export default function App({ authUser }: { authUser?: AuthUser | null }) {
  const [activePage, setActivePage] = useState<AppPage>("printers")
  const [templatePrintRevision, setTemplatePrintRevision] = useState(0)
  const controller = useDeepprintController()

  let activePageContent: ReactNode
  if (activePage === "print") {
    activePageContent = (
      <PrintPage
        controller={controller}
        templatePrintRevision={templatePrintRevision}
        onNavigate={setActivePage}
      />
    )
  } else if (activePage === "templates") {
    activePageContent = (
      <TemplatesPage
        controller={controller}
        onNavigatePrint={() => {
          setTemplatePrintRevision((current) => current + 1)
          setActivePage("print")
        }}
      />
    )
  } else if (activePage === "printers") {
    activePageContent = (
      <div className="min-h-0 flex-1 overflow-y-auto bg-background px-6 py-6">
        <div className="mx-auto max-w-5xl">
          <PrinterManagementPage
            controller={controller}
            showHeader={false}
          />
        </div>
      </div>
    )
  } else if (activePage === "history") {
    activePageContent = (
      <div className="min-h-0 flex-1 overflow-y-auto bg-background px-3 py-4 sm:px-6 sm:py-6">
        <div className="mx-auto max-w-5xl">
          <PrintHistoryPage
            controller={controller}
            showHeader={false}
          />
        </div>
      </div>
    )
  } else if (activePage === "users") {
    activePageContent = (
      <UsersPage
        baseUrl={controller.ui.baseUrl}
        currentUser={authUser ?? null}
      />
    )
  } else {
    activePageContent = <ApiKeysPage baseUrl={controller.ui.baseUrl} />
  }

  return (
    <TooltipProvider>
      <SettingsProvider>
        <SidebarProvider
          style={
            {
              "--sidebar-width": "calc(var(--spacing) * 72)",
              "--header-height": "calc(var(--spacing) * 12)",
            } as CSSProperties
          }
        >
          <AppSidebar
            variant="inset"
            activePage={activePage}
            authUser={authUser ?? null}
            onNavigate={setActivePage}
          />
          <SidebarInset>
            <SiteHeader activePage={activePage} />
            <Suspense fallback={<PageLoadingState />}>
              {activePageContent}
            </Suspense>
          </SidebarInset>
        </SidebarProvider>
        <SettingsOverlayHost controller={controller} />
      </SettingsProvider>
    </TooltipProvider>
  )
}

function SettingsOverlayHost({
  controller,
}: {
  controller: ReturnType<typeof useDeepprintController>
}) {
  const { isOpen } = useSettings()

  if (!isOpen) return null

  return (
    <Suspense fallback={null}>
      <SettingsOverlay controller={controller} />
    </Suspense>
  )
}

function PageLoadingState() {
  const { t } = useI18n()

  return (
    <div className="flex min-h-0 flex-1 items-center justify-center bg-background px-6 py-10">
      <div className="rounded-xl border bg-card px-5 py-4 text-sm text-muted-foreground shadow-sm">
        {t("common.loadingPage")}
      </div>
    </div>
  )
}
