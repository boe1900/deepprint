import * as React from "react"

import { NavMain } from "@/components/nav-main"
import { NavUser } from "@/components/nav-user"
import type { AppPage } from "@/App"
import type { AuthUser } from "@/features/auth/types"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"
import {
  FileTextIcon,
  HistoryIcon,
  KeyRoundIcon,
  PrinterIcon,
  SendIcon,
  UsersIcon,
} from "lucide-react"

const data = {
  navMain: [
    {
      id: "printers" as const,
      title: "打印机",
      icon: <PrinterIcon />,
    },
    {
      id: "print" as const,
      title: "打印中心",
      icon: <SendIcon />,
    },
    {
      id: "templates" as const,
      title: "模板管理",
      icon: <FileTextIcon />,
    },
    {
      id: "history" as const,
      title: "打印记录",
      icon: <HistoryIcon />,
    },
    {
      id: "users" as const,
      title: "用户管理",
      icon: <UsersIcon />,
      adminOnly: true,
    },
    {
      id: "apiKeys" as const,
      title: "API Key",
      icon: <KeyRoundIcon />,
      adminOnly: true,
    },
  ],
}

export function AppSidebar({
  activePage,
  authUser,
  onNavigate,
  ...props
}: React.ComponentProps<typeof Sidebar> & {
  activePage: AppPage
  authUser: AuthUser | null
  onNavigate: (page: AppPage) => void
}) {
  const navItems = data.navMain.filter(
    (item) => !("adminOnly" in item) || authUser?.role === "admin"
  )

  return (
    <Sidebar collapsible="offcanvas" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              className="data-[slot=sidebar-menu-button]:p-1.5!"
              render={
                <a href="#" className="flex items-center gap-2" />
              }
            >
              <img
                src="/favicon.svg"
                alt=""
                aria-hidden="true"
                className="size-6 rounded-md"
              />
              <span className="text-base font-semibold tracking-tight">
                DeepPrint
              </span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <NavMain
          activePage={activePage}
          items={navItems}
          onNavigate={onNavigate}
        />
      </SidebarContent>
      <SidebarFooter>
        {authUser ? <NavUser user={authUser} /> : null}
      </SidebarFooter>
    </Sidebar>
  )
}
