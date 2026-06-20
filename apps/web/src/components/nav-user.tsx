import { useMemo } from "react"
import { useNavigate } from "@tanstack/react-router"
import { useMutation, useQueryClient } from "@tanstack/react-query"
import {
  ChevronsUpDownIcon,
  KeyRoundIcon,
  LanguagesIcon,
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
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
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
import { localeLabels, locales, useI18n, type Locale } from "@/i18n"

export function NavUser({ user }: { user: AuthUser }) {
  const { locale, setLocale, t } = useI18n()
  const { isMobile } = useSidebar()
  const { open: openSettings } = useSettings()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const baseUrl = useMemo(() => getAuthBaseUrl(), [])
  const name = user.display_name || user.username
  const subtitle = user.email || t("auth.localAdmin")
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
                <span>{user.role === "admin" ? t("auth.admin") : user.role}</span>
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
                <span>{t("auth.changePassword")}</span>
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuLabel>{t("language.label")}</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={locale}
                onValueChange={(value) => setLocale(value as Locale)}
              >
                {locales.map((item) => (
                  <DropdownMenuRadioItem key={item} value={item}>
                    <LanguagesIcon />
                    <span>{localeLabels[item]}</span>
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem onClick={() => openSettings("packages")}>
                <PackageIcon />
                <span>{t("settings.packages.label")}</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openSettings("fonts")}>
                <TypeIcon />
                <span>{t("settings.fonts.label")}</span>
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => openSettings()}>
                <SettingsIcon />
                <span>{t("settings.open")}</span>
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
                <span>{t("auth.logout")}</span>
              </DropdownMenuItem>
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
