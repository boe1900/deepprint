import type { ReactNode } from "react"
import { useEffect, useMemo } from "react"
import { useNavigate } from "@tanstack/react-router"
import { useQuery } from "@tanstack/react-query"
import { Loader2Icon, PrinterIcon, RefreshCwIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import { createAuthMeQueryOptions } from "@/features/auth/queries"
import { getAuthBaseUrl } from "@/features/auth/session"
import type { AuthUser } from "@/features/auth/types"

export function AuthGate({
  children,
}: {
  children: (auth: { user: AuthUser | null; loginEnabled: boolean }) => ReactNode
}) {
  const navigate = useNavigate()
  const baseUrl = useMemo(() => getAuthBaseUrl(), [])
  const meQuery = useQuery({
    ...createAuthMeQueryOptions(baseUrl),
    staleTime: 5_000,
  })

  useEffect(() => {
    if (!meQuery.data) return
    if (meQuery.data.login_enabled && !meQuery.data.authenticated) {
      void navigate({ to: "/login" })
      return
    }
    if (meQuery.data.authenticated && meQuery.data.user?.must_change_password) {
      void navigate({ to: "/change-password" })
    }
  }, [
    meQuery.data,
    meQuery.data?.authenticated,
    meQuery.data?.login_enabled,
    meQuery.data?.user?.must_change_password,
    navigate,
  ])

  if (meQuery.isPending) {
    return <AuthStatusScreen state="loading" />
  }

  if (meQuery.isError) {
    const message =
      meQuery.error instanceof Error ? meQuery.error.message : "登录状态读取失败"
    return (
      <AuthStatusScreen
        state="error"
        message={message}
        onRetry={() => void meQuery.refetch()}
      />
    )
  }

  if (!meQuery.data.login_enabled) {
    return <AuthBootstrapScreen />
  }

  if (
    (meQuery.data.login_enabled && !meQuery.data.authenticated) ||
    (meQuery.data.authenticated && meQuery.data.user?.must_change_password)
  ) {
    return <AuthStatusScreen state="loading" />
  }

  return (
    <>
      {children({
        user: meQuery.data.user,
        loginEnabled: meQuery.data.login_enabled,
      })}
    </>
  )
}

function AuthBootstrapScreen() {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center bg-muted p-6">
      <div className="flex w-full max-w-lg flex-col gap-4 rounded-2xl border bg-background p-6 text-center shadow-sm">
        <div className="mx-auto flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <PrinterIcon className="size-5" />
        </div>
        <div className="space-y-2">
          <h1 className="text-lg font-semibold">还没有可登录的管理员账号</h1>
          <p className="text-sm leading-6 text-muted-foreground">
            当前实例还没有初始化本地管理员，控制台暂时无法登录。
          </p>
        </div>
        <div className="rounded-xl border bg-muted/40 px-4 py-3 text-left text-sm text-muted-foreground">
          请先在项目根目录创建或更新 <code>.env</code>，至少设置
          <code>DEEPPRINT_INITIAL_ADMIN_PASSWORD</code>。如果是 Docker 启动，更新后重新执行
          <code>docker compose up -d</code>；如果是本地开发，重启 <code>bun run server:dev</code>。
        </div>
      </div>
    </main>
  )
}

function AuthStatusScreen({
  state,
  message,
  onRetry,
}: {
  state: "loading" | "error"
  message?: string
  onRetry?: () => void
}) {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center bg-muted p-6">
      <div className="flex w-full max-w-sm flex-col items-center gap-4 text-center">
        <div className="flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <PrinterIcon className="size-5" />
        </div>
        {state === "loading" ? (
          <Loader2Icon className="size-5 animate-spin text-muted-foreground" />
        ) : (
          <>
            <div className="space-y-1">
              <h1 className="text-base font-medium">无法读取登录状态</h1>
              {message ? (
                <p className="text-sm text-muted-foreground">{message}</p>
              ) : null}
            </div>
            <Button type="button" variant="outline" onClick={onRetry}>
              <RefreshCwIcon data-icon="inline-start" />
              重试
            </Button>
          </>
        )}
      </div>
    </main>
  )
}
