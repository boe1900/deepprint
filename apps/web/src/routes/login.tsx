import { useEffect, useMemo, useState } from "react"
import { createFileRoute, useNavigate } from "@tanstack/react-router"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Loader2Icon, PrinterIcon } from "lucide-react"

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
import { login } from "@/features/auth/api"
import { authQueryKeys, createAuthMeQueryOptions } from "@/features/auth/queries"
import { getAuthBaseUrl } from "@/features/auth/session"
import { useI18n } from "@/i18n"

export const Route = createFileRoute("/login")({
  component: LoginPage,
})

function LoginPage() {
  const { t } = useI18n()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const baseUrl = useMemo(() => getAuthBaseUrl(), [])
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")

  const meQuery = useQuery({
    ...createAuthMeQueryOptions(baseUrl),
    staleTime: 5_000,
  })

  const loginMutation = useMutation({
    mutationFn: () => login(baseUrl, username, password),
    onSuccess: async (response) => {
      queryClient.setQueryData(authQueryKeys.me(baseUrl), {
        authenticated: true,
        login_enabled: true,
        user: response.user,
        expires_at: response.expires_at,
      })
      await navigate({
        to: response.user.must_change_password ? "/change-password" : "/",
      })
    },
  })

  useEffect(() => {
    if (meQuery.data?.authenticated) {
      void navigate({
        to: meQuery.data.user?.must_change_password ? "/change-password" : "/",
      })
    }
  }, [
    meQuery.data?.authenticated,
    meQuery.data?.user?.must_change_password,
    navigate,
  ])

  const errorMessage =
    loginMutation.error instanceof Error ? loginMutation.error.message : null
  const submitting = loginMutation.isPending

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
            <CardTitle className="text-lg">{t("auth.loginTitle")}</CardTitle>
            <CardDescription>{t("auth.loginDescription")}</CardDescription>
          </CardHeader>
          <CardContent>
            <form
              className="flex flex-col gap-4"
              onSubmit={(event) => {
                event.preventDefault()
                if (!username.trim() || !password) return
                loginMutation.mutate()
              }}
            >
              <div className="flex flex-col gap-2">
                <Label htmlFor="username">{t("auth.username")}</Label>
                <Input
                  id="username"
                  autoComplete="username"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                  disabled={submitting}
                  placeholder="admin"
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="password">{t("auth.password")}</Label>
                <Input
                  id="password"
                  type="password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  disabled={submitting}
                />
              </div>

              {errorMessage ? (
                <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {errorMessage}
                </div>
              ) : null}

              <Button
                type="submit"
                className="w-full"
                disabled={submitting || !username.trim() || !password}
              >
                {submitting ? (
                  <Loader2Icon
                    data-icon="inline-start"
                    className="animate-spin"
                  />
                ) : null}
                {t("auth.login")}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    </main>
  )
}
