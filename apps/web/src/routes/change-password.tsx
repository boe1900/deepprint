import { useEffect, useMemo, useState } from "react"
import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { KeyRoundIcon, Loader2Icon, PrinterIcon } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { changePassword } from "@/features/auth/api"
import { authQueryKeys, createAuthMeQueryOptions } from "@/features/auth/queries"
import { getAuthBaseUrl } from "@/features/auth/session"
import { useI18n } from "@/i18n"

export const Route = createFileRoute("/change-password")({
  validateSearch: (search: Record<string, unknown>) => ({
    mode: search.mode === "account" ? "account" : undefined,
  }),
  component: ChangePasswordPage,
})

function ChangePasswordPage() {
  const { t } = useI18n()
  const search = Route.useSearch()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const baseUrl = useMemo(() => getAuthBaseUrl(), [])
  const [currentPassword, setCurrentPassword] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [confirmPassword, setConfirmPassword] = useState("")

  const accountMode = search.mode === "account"

  const meQuery = useQuery({
    ...createAuthMeQueryOptions(baseUrl),
    staleTime: 5_000,
  })

  const changePasswordMutation = useMutation({
    mutationFn: () => changePassword(baseUrl, currentPassword, newPassword),
    onSuccess: async (response) => {
      queryClient.setQueryData(authQueryKeys.me(baseUrl), {
        authenticated: true,
        login_enabled: true,
        user: response.user,
        expires_at: response.expires_at,
      })
      await navigate({ to: "/" })
    },
  })

  useEffect(() => {
    if (!meQuery.data) return
    if (meQuery.data.login_enabled && !meQuery.data.authenticated) {
      void navigate({ to: "/login" })
      return
    }
    if (
      meQuery.data.authenticated &&
      !meQuery.data.user?.must_change_password &&
      !accountMode
    ) {
      void navigate({ to: "/" })
    }
  }, [
    accountMode,
    meQuery.data,
    meQuery.data?.authenticated,
    meQuery.data?.login_enabled,
    meQuery.data?.user?.must_change_password,
    navigate,
  ])

  const localError =
    newPassword && newPassword.length < 8
      ? t("auth.passwordTooShort")
      : confirmPassword && newPassword !== confirmPassword
        ? t("auth.passwordMismatch")
        : null
  const mutationError =
    changePasswordMutation.error instanceof Error
      ? changePasswordMutation.error.message
      : null
  const errorMessage = localError || mutationError
  const submitting = changePasswordMutation.isPending
  const canSubmit =
    currentPassword.length > 0 &&
    newPassword.length >= 8 &&
    newPassword === confirmPassword &&
    !submitting

  return (
    <main className="flex min-h-svh flex-col items-center justify-center bg-muted p-6 md:p-10">
      <div className="flex w-full max-w-sm flex-col gap-6">
        <div className="flex items-center justify-center gap-2 text-sm font-medium">
          <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <PrinterIcon className="size-4" />
          </div>
          <span>DeepPrint Studio</span>
        </div>

        <Card className="shadow-sm">
          <CardHeader className="text-center">
            <div className="mx-auto mb-1 flex size-8 items-center justify-center rounded-lg bg-muted text-muted-foreground">
              <KeyRoundIcon className="size-4" />
            </div>
            <CardTitle className="text-lg">
              {accountMode ? t("auth.changePassword") : t("auth.changeInitialPassword")}
            </CardTitle>
            <CardDescription>
              {accountMode
                ? t("auth.updateAccountPassword")
                : t("auth.updateInitialPassword")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form
              className="flex flex-col gap-4"
              onSubmit={(event) => {
                event.preventDefault()
                if (!canSubmit) return
                changePasswordMutation.mutate()
              }}
            >
              <div className="flex flex-col gap-2">
                <Label htmlFor="current-password">{t("auth.currentPassword")}</Label>
                <Input
                  id="current-password"
                  type="password"
                  autoComplete="current-password"
                  value={currentPassword}
                  onChange={(event) => setCurrentPassword(event.target.value)}
                  disabled={submitting}
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="new-password">{t("auth.newPassword")}</Label>
                <Input
                  id="new-password"
                  type="password"
                  autoComplete="new-password"
                  value={newPassword}
                  onChange={(event) => setNewPassword(event.target.value)}
                  disabled={submitting}
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="confirm-password">{t("auth.confirmNewPassword")}</Label>
                <Input
                  id="confirm-password"
                  type="password"
                  autoComplete="new-password"
                  value={confirmPassword}
                  onChange={(event) => setConfirmPassword(event.target.value)}
                  disabled={submitting}
                />
              </div>

              {errorMessage ? (
                <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {errorMessage}
                </div>
              ) : null}

              <Button type="submit" className="w-full" disabled={!canSubmit}>
                {submitting ? (
                  <Loader2Icon
                    data-icon="inline-start"
                    className="animate-spin"
                  />
                ) : null}
                {t("auth.updatePassword")}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
  )
}
