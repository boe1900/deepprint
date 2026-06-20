import type { ReactNode } from "react"
import { useEffect, useMemo } from "react"
import { useNavigate } from "@tanstack/react-router"
import { useQuery } from "@tanstack/react-query"
import { Loader2Icon, PrinterIcon, RefreshCwIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import { createAuthMeQueryOptions } from "@/features/auth/queries"
import { getAuthBaseUrl } from "@/features/auth/session"
import type { AuthUser } from "@/features/auth/types"
import { useI18n } from "@/i18n"

export function AuthGate({
  children,
}: {
  children: (auth: { user: AuthUser | null; loginEnabled: boolean }) => ReactNode
}) {
  const { t } = useI18n()
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
      meQuery.error instanceof Error ? meQuery.error.message : t("auth.loginStatusReadFailed")
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
  const { t } = useI18n()

  return (
    <main className="flex min-h-svh flex-col items-center justify-center bg-muted p-6">
      <div className="flex w-full max-w-lg flex-col gap-4 rounded-2xl border bg-background p-6 text-center shadow-sm">
        <div className="mx-auto flex size-9 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <PrinterIcon className="size-5" />
        </div>
        <div className="space-y-2">
          <h1 className="text-lg font-semibold">{t("auth.noAdminTitle")}</h1>
          <p className="text-sm leading-6 text-muted-foreground">
            {t("auth.noAdminDescription")}
          </p>
        </div>
        <div className="rounded-xl border bg-muted/40 px-4 py-3 text-left text-sm text-muted-foreground">
          {t("auth.noAdminSetup")}
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
  const { t } = useI18n()

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
              <h1 className="text-base font-medium">{t("auth.loginStatusError")}</h1>
              {message ? (
                <p className="text-sm text-muted-foreground">{message}</p>
              ) : null}
            </div>
            <Button type="button" variant="outline" onClick={onRetry}>
              <RefreshCwIcon data-icon="inline-start" />
              {t("common.retry")}
            </Button>
          </>
        )}
      </div>
    </main>
  )
}
