import { Separator } from "@/components/ui/separator"
import { SidebarTrigger } from "@/components/ui/sidebar"
import type { AppPage } from "@/App"

const pageTitle: Record<AppPage, string> = {
  print: "打印中心",
  templates: "模板管理",
  printers: "打印机",
  history: "打印记录",
  users: "用户管理",
  apiKeys: "API Key",
}

export function SiteHeader({ activePage }: { activePage: AppPage }) {
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
        <h1 className="text-base font-medium">{pageTitle[activePage]}</h1>
      </div>
    </header>
  )
}
