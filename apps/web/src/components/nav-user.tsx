import { useMemo } from "react"
import { useNavigate } from "@tanstack/react-router"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import {
  ChevronsUpDownIcon,
  KeyRoundIcon,
  Loader2Icon,
  LogOutIcon,
  PackageIcon,
  SettingsIcon,
  ShieldCheckIcon,
  TypeIcon,
} from "lucide-react"

import {
  Avatar,
  AvatarFallback,
} from "@/components/ui/avatar"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar"
import { logout } from "@/features/auth/api"
import { authQueryKeys } from "@/features/auth/queries"
import { getAuthBaseUrl } from "@/features/auth/session"
import type { AuthUser } from "@/features/auth/types"
import { userAvatarInitial, userAvatarTone } from "@/features/auth/user-avatar"
import { useSettings } from "@/hooks/use-settings"

export function NavUser({ user }: { user: AuthUser }) {
  const { isMobile } = useSidebar()
  const { open: openSettings } = useSettings()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const baseUrl = useMemo(() => getAuthBaseUrl(), [])
  const name = user.display_name || user.username
  const subtitle = user.email || "本地管理员"
  const avatarClassName = userAvatarTone(name)
  const avatarInitial = userAvatarInitial(name)

  const logoutMutation = useMutation({
    mutationFn: () => logout(baseUrl),
    onSuccess: async () => {
      queryClient.setQueryData(authQueryKeys.me(baseUrl), {
        authenticated: false,
        login_enabled: true,
        user: null,
        expires_at: null,
      })
      await navigate({ to: "/login" })
    },
  })

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <SidebarMenuButton
                size="lg"
                className="data-popup-open:bg-sidebar-accent data-popup-open:text-sidebar-accent-foreground"
              />
            }
          >
            <Avatar className={avatarClassName}>
              <AvatarFallback className="bg-transparent text-sm font-semibold text-inherit">
                {avatarInitial}
              </AvatarFallback>
            </Avatar>
            <div className="grid flex-1 text-left text-sm leading-tight">
              <span className="truncate font-medium">{name}</span>
              <span className="truncate text-xs text-muted-foreground">
                {subtitle}
              </span>
            </div>
            <ChevronsUpDownIcon className="ml-auto size-4" />
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-(--anchor-width) min-w-56 rounded-lg"
            side={isMobile ? "bottom" : "right"}
            align="end"
            sideOffset={4}
          >
            <DropdownMenuGroup>
              <DropdownMenuLabel className="p-0 font-normal">
                <div className="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
                  <Avatar className={avatarClassName}>
                    <AvatarFallback className="bg-transparent text-sm font-semibold text-inherit">
                      {avatarInitial}
                    </AvatarFallback>
                  </Avatar>
                  <div className="grid flex-1 text-left text-sm leading-tight">
                    <span className="truncate font-medium">{name}</span>
                    <span className="truncate text-xs text-muted-foreground">
                      {subtitle}
                    </span>
                  </div>
                </div>
              </DropdownMenuLabel>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem disabled>
                <ShieldCheckIcon />
                <span>{user.role === "admin" ? "管理员" : user.role}</span>
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() =>
                  void navigate({
                    to: "/change-password",
                    search: { mode: "account" },
                  })
                }
              >
                <KeyRoundIcon />
                <span>修改密码</span>
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem onClick={() => openSettings("packages")}>
                <PackageIcon />
                <span>Typst 包管理</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openSettings("fonts")}>
                <TypeIcon />
                <span>字体管理</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openSettings()}>
                <SettingsIcon />
                <span>打开设置</span>
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem
                variant="destructive"
                disabled={logoutMutation.isPending}
                onClick={() => logoutMutation.mutate()}
              >
                {logoutMutation.isPending ? (
                  <Loader2Icon className="animate-spin" />
                ) : (
                  <LogOutIcon />
                )}
                <span>退出登录</span>
              </DropdownMenuItem>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
