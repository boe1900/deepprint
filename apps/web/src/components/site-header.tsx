import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import type { AppPage } from "@/App"
import { useI18n, type MessageKey } from "@/i18n"

const pageTitleKey: Record<AppPage, MessageKey> = {
  print: "nav.print",
  templates: "nav.templates",
  printers: "nav.printers",
  history: "nav.history",
  users: "nav.users",
  apiKeys: "nav.apiKeys",
}

export function SiteHeader({ activePage }: { activePage: AppPage }) {
  const { t } = useI18n()

  return (
    <header
      className="flex h-(--header-height) shrink-0 items-center gap-2 border-b transition-[width,height] ease-linear group-has-data-[collapsible=icon]/sidebar-wrapper:h-(--header-height)"
    >
      <div className="flex w-full items-center gap-1 px-4 lg:gap-2 lg:px-6">
        <SidebarTrigger className="-ml-1" />
        <Separator
          orientation="vertical"
          className="mx-2 data-[orientation=vertical]:h-4 data-[orientation=vertical]:self-center"
        />
        <h1 className="text-base font-medium">{t(pageTitleKey[activePage])}</h1>
      </div>
    </header>
  )
}
