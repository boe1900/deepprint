import { useMemo, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  AlertCircleIcon,
  InboxIcon,
  KeyRoundIcon,
  Loader2Icon,
  MailIcon,
  PlusIcon,
  RefreshCwIcon,
  SearchIcon,
  ShieldAlertIcon,
  ShieldCheckIcon,
  Trash2Icon,
  UserCheckIcon,
  UserXIcon,
} from "lucide-react"

import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  createUser,
  deleteUser,
  resetUserPassword,
  updateUser,
} from "@/features/auth/api"
import { authQueryKeys, createAuthUsersQueryOptions } from "@/features/auth/queries"
import type { AuthUser, AuthUserRole, AuthUserStatus } from "@/features/auth/types"
import { userAvatarInitial, userAvatarTone } from "@/features/auth/user-avatar"
import { useI18n, type MessageKey } from "@/i18n"
import { cn } from "@/lib/utils"

type UsersPageProps = {
  baseUrl: string
  currentUser: AuthUser | null
}

type CreateUserForm = {
  username: string
  displayName: string
  email: string
  role: AuthUserRole
  password: string
}

const initialCreateForm: CreateUserForm = {
  username: "",
  displayName: "",
  email: "",
  role: "operator",
  password: "",
}

export function UsersPage({ baseUrl, currentUser }: UsersPageProps) {
  const { t } = useI18n()
  const queryClient = useQueryClient()
  const [createOpen, setCreateOpen] = useState(false)
  const [createForm, setCreateForm] = useState<CreateUserForm>(initialCreateForm)
  const [resetTarget, setResetTarget] = useState<AuthUser | null>(null)
  const [resetPassword, setResetPassword] = useState("")
  const [deleteTarget, setDeleteTarget] = useState<AuthUser | null>(null)
  const [search, setSearch] = useState("")

  const usersQueryOptions = useMemo(
    () => createAuthUsersQueryOptions(baseUrl),
    [baseUrl]
  )
  const usersQuery = useQuery(usersQueryOptions)

  const createMutation = useMutation({
    mutationFn: () =>
      createUser(baseUrl, {
        username: createForm.username,
        password: createForm.password,
        email: createForm.email || null,
        display_name: createForm.displayName || null,
        role: createForm.role,
      }),
    onSuccess: async () => {
      setCreateForm(initialCreateForm)
      setCreateOpen(false)
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.users(baseUrl),
      })
    },
  })

  const statusMutation = useMutation({
    mutationFn: ({
      user,
      status,
    }: {
      user: AuthUser
      status: AuthUserStatus
    }) => updateUser(baseUrl, user.id, { status }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.users(baseUrl),
      })
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.me(baseUrl),
      })
    },
  })

  const resetMutation = useMutation({
    mutationFn: () => {
      if (!resetTarget) throw new Error(t("users.selectUser"))
      return resetUserPassword(baseUrl, resetTarget.id, resetPassword)
    },
    onSuccess: async () => {
      setResetTarget(null)
      setResetPassword("")
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.users(baseUrl),
      })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: (user: AuthUser) => deleteUser(baseUrl, user.id),
    onSuccess: async () => {
      setDeleteTarget(null)
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.users(baseUrl),
      })
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.me(baseUrl),
      })
    },
  })

  const users = usersQuery.data?.users ?? []
  const filteredUsers = useMemo(() => {
    const keyword = search.trim().toLowerCase()
    if (!keyword) return users
    return users.filter((user) =>
      [
        user.username,
        user.display_name,
        user.email ?? "",
        roleLabel(user.role, t),
        statusLabel(user.status, t),
      ]
        .join(" ")
        .toLowerCase()
        .includes(keyword)
    )
  }, [search, t, users])
  const adminUsers = users.filter((user) => user.role === "admin").length
  const createError =
    createMutation.error instanceof Error ? createMutation.error.message : null
  const resetError =
    resetMutation.error instanceof Error ? resetMutation.error.message : null
  const statusError =
    statusMutation.error instanceof Error ? statusMutation.error.message : null
  const deleteError =
    deleteMutation.error instanceof Error ? deleteMutation.error.message : null
  const canCreate =
    createForm.username.trim().length > 0 &&
    createForm.password.length >= 8 &&
    !createMutation.isPending
  const canReset =
    Boolean(resetTarget) && resetPassword.length >= 8 && !resetMutation.isPending

  return (
    <div className="min-h-0 flex-1 overflow-y-auto bg-background px-3 py-4 sm:px-6 sm:py-6">
      <div className="mx-auto flex max-w-6xl flex-col gap-5">
        <header className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <div className="min-w-0">
            <h1 className="font-heading text-2xl font-semibold tracking-tight">
              {t("users.title")}
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              {t("users.description")}
            </p>
          </div>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <div className="relative sm:w-64">
              <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                className="pl-8"
                placeholder={t("users.searchPlaceholder")}
              />
            </div>
            <Button
              type="button"
              variant="outline"
              size="icon"
              title={t("common.refresh")}
              onClick={() => void usersQuery.refetch()}
              disabled={usersQuery.isFetching}
            >
              <RefreshCwIcon
                className={usersQuery.isFetching ? "animate-spin" : undefined}
              />
              <span className="sr-only">{t("common.refresh")}</span>
            </Button>
            <Button type="button" onClick={() => setCreateOpen(true)}>
              <PlusIcon data-icon="inline-start" />
              {t("users.newUser")}
            </Button>
          </div>
        </header>

        {usersQuery.error ? (
          <ErrorNotice>
            {usersQuery.error instanceof Error
              ? usersQuery.error.message
              : t("users.loadFailed")}
          </ErrorNotice>
        ) : null}
        {statusError ? <ErrorNotice>{statusError}</ErrorNotice> : null}

        <section className="overflow-hidden rounded-xl border bg-card shadow-sm">
          <div className="hidden md:block">
            <Table>
              <TableHeader className="bg-muted/40">
                <TableRow>
                  <TableHead className="pl-6">{t("users.headUser")}</TableHead>
                  <TableHead className="w-32">{t("users.headRole")}</TableHead>
                  <TableHead className="w-32">{t("users.headStatus")}</TableHead>
                  <TableHead className="w-40">{t("users.headSecurity")}</TableHead>
                  <TableHead className="w-48">{t("users.headUpdated")}</TableHead>
                  <TableHead className="w-40 pr-6 text-right">{t("users.headActions")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {usersQuery.isPending ? (
                  <LoadingRow />
                ) : filteredUsers.length === 0 ? (
                  <EmptyRow search={search} />
                ) : (
                  filteredUsers.map((user) => (
                    <UserTableRow
                      key={user.id}
                      currentUserId={currentUser?.id ?? null}
                      deletePending={
                        deleteMutation.isPending &&
                        deleteMutation.variables?.id === user.id
                      }
                      onDelete={setDeleteTarget}
                      onReset={(target) => {
                        setResetTarget(target)
                        setResetPassword("")
                      }}
                      onToggleStatus={(target, status) =>
                        statusMutation.mutate({ user: target, status })
                      }
                      statusMutation={statusMutation}
                      user={user}
                    />
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          <div className="divide-y md:hidden">
            {usersQuery.isPending ? (
              Array.from({ length: 4 }).map((_, index) => (
                <div key={index} className="space-y-3 px-4 py-4">
                  <div className="h-4 w-2/3 animate-pulse rounded bg-muted" />
                  <div className="h-3 w-1/2 animate-pulse rounded bg-muted" />
                  <div className="h-8 w-full animate-pulse rounded bg-muted" />
                </div>
              ))
            ) : filteredUsers.length === 0 ? (
              <EmptyState search={search} />
            ) : (
              filteredUsers.map((user) => (
                <UserMobileCard
                  key={user.id}
                  currentUserId={currentUser?.id ?? null}
                  deletePending={
                    deleteMutation.isPending &&
                    deleteMutation.variables?.id === user.id
                  }
                  onDelete={setDeleteTarget}
                  onReset={(target) => {
                    setResetTarget(target)
                    setResetPassword("")
                  }}
                  onToggleStatus={(target, status) =>
                    statusMutation.mutate({ user: target, status })
                  }
                  statusMutation={statusMutation}
                  user={user}
                />
              ))
            )}
          </div>

          <div className="flex items-center justify-between border-t bg-muted/20 px-4 py-3 text-xs text-muted-foreground sm:px-6">
            <span>
              {search.trim()
                ? t("users.footerCountFiltered", { count: filteredUsers.length, total: users.length })
                : t("users.footerCount", { count: filteredUsers.length })}
            </span>
            <span>{t("users.adminCount", { count: adminUsers })}</span>
          </div>
        </section>
      </div>

      <Sheet open={createOpen} onOpenChange={setCreateOpen}>
        <SheetContent className="w-full max-w-none data-[side=right]:w-full data-[side=right]:sm:max-w-md">
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle>{t("users.newUser")}</SheetTitle>
            <SheetDescription>
              {t("users.createDescription")}
            </SheetDescription>
          </SheetHeader>
          <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
            <LabeledInput
              id="create-username"
              label={t("auth.username")}
              required
              autoComplete="username"
              placeholder={t("auth.username")}
              value={createForm.username}
              disabled={createMutation.isPending}
              onChange={(value) =>
                setCreateForm((current) => ({ ...current, username: value }))
              }
            />
            <LabeledInput
              id="create-display-name"
              label={t("users.displayName")}
              placeholder={t("users.displayNamePlaceholder")}
              value={createForm.displayName}
              disabled={createMutation.isPending}
              onChange={(value) =>
                setCreateForm((current) => ({ ...current, displayName: value }))
              }
            />
            <LabeledInput
              id="create-email"
              label={t("users.email")}
              type="email"
              autoComplete="email"
              placeholder="example@corp.com"
              value={createForm.email}
              disabled={createMutation.isPending}
              onChange={(value) =>
                setCreateForm((current) => ({ ...current, email: value }))
              }
            />
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="create-role">{t("users.roleAssignment")}</Label>
              <Select
                value={createForm.role}
                onValueChange={(value) =>
                  setCreateForm((current) => ({
                    ...current,
                    role: (value ?? "operator") as AuthUserRole,
                  }))
                }
              >
                <SelectTrigger id="create-role" className="w-full">
                  <SelectValue placeholder={t("users.selectRole")} />
                </SelectTrigger>
                <SelectContent align="start">
                  <SelectGroup>
                    <SelectItem value="operator">{t("users.roleOperatorOption")}</SelectItem>
                    <SelectItem value="admin">{t("users.roleAdminOption")}</SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
            </div>
            <div className="border-t pt-5">
              <LabeledInput
                id="create-password"
                label={t("users.initialPassword")}
                required
                type="password"
                autoComplete="new-password"
                placeholder={t("users.initialPasswordPlaceholder")}
                value={createForm.password}
                disabled={createMutation.isPending}
                onChange={(value) =>
                  setCreateForm((current) => ({ ...current, password: value }))
                }
              />
            </div>
            {createError ? <ErrorNotice>{createError}</ErrorNotice> : null}
          </div>
          <SheetFooter className="flex-col-reverse border-t bg-muted/20 px-6 py-4 sm:flex-row sm:justify-end">
            <Button
              type="button"
              variant="outline"
              disabled={createMutation.isPending}
              onClick={() => setCreateOpen(false)}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              disabled={!canCreate}
              onClick={() => createMutation.mutate()}
            >
              {createMutation.isPending ? (
                <Loader2Icon data-icon="inline-start" className="animate-spin" />
              ) : (
                <PlusIcon data-icon="inline-start" />
              )}
              {t("users.createUser")}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      <Sheet
        open={Boolean(resetTarget)}
        onOpenChange={(open) => {
          if (!open) {
            setResetTarget(null)
            setResetPassword("")
          }
        }}
      >
        <SheetContent className="w-full max-w-none data-[side=right]:w-full data-[side=right]:sm:max-w-md">
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle>{t("users.resetTitle")}</SheetTitle>
            <SheetDescription>
              {resetTarget
                ? t("users.resetDescriptionWithName", { name: resetTarget.display_name || resetTarget.username })
                : t("users.resetDescription")}
            </SheetDescription>
          </SheetHeader>
          <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
            <div className="flex gap-3 rounded-lg border border-amber-100 bg-amber-50/50 p-3 text-sm text-amber-800">
              <AlertCircleIcon className="mt-0.5 size-5 shrink-0 text-amber-500" />
              <p className="leading-relaxed">
                {t("users.resetWarningBefore")}
                <strong className="font-semibold">{t("users.resetWarningStrong")}</strong>
                {t("users.resetWarningAfter")}
              </p>
            </div>
            <LabeledInput
              id="reset-password"
              label={t("users.newTemporaryPassword")}
              required
              type="password"
              autoComplete="new-password"
              placeholder={t("users.newPasswordPlaceholder")}
              value={resetPassword}
              disabled={resetMutation.isPending}
              onChange={setResetPassword}
            />
            {resetError ? <ErrorNotice>{resetError}</ErrorNotice> : null}
          </div>
          <SheetFooter className="flex-col-reverse border-t bg-muted/20 px-6 py-4 sm:flex-row sm:justify-end">
            <Button
              type="button"
              variant="outline"
              disabled={resetMutation.isPending}
              onClick={() => {
                setResetTarget(null)
                setResetPassword("")
              }}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              disabled={!canReset}
              onClick={() => resetMutation.mutate()}
            >
              {resetMutation.isPending ? (
                <Loader2Icon data-icon="inline-start" className="animate-spin" />
              ) : (
                <KeyRoundIcon data-icon="inline-start" />
              )}
              {t("users.confirmReset")}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>

      <Sheet
        open={Boolean(deleteTarget)}
        onOpenChange={(open) => {
          if (!open && !deleteMutation.isPending) {
            setDeleteTarget(null)
          }
        }}
      >
        <SheetContent className="w-full max-w-none data-[side=right]:w-full data-[side=right]:sm:max-w-md">
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle>{t("users.deleteTitle")}</SheetTitle>
            <SheetDescription>
              {deleteTarget
                ? t("users.deleteDescriptionWithName", { name: deleteTarget.display_name || deleteTarget.username })
                : t("users.deleteDescription")}
            </SheetDescription>
          </SheetHeader>
          <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
            <div className="flex gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-3 text-sm text-destructive">
              <AlertCircleIcon className="mt-0.5 size-5 shrink-0" />
              <p className="leading-relaxed">
                {t("users.deleteWarning")}
              </p>
            </div>
            {deleteTarget ? (
              <div className="rounded-lg border bg-muted/30 p-3">
                <UserIdentity isSelf={false} user={deleteTarget} />
              </div>
            ) : null}
            {deleteError ? <ErrorNotice>{deleteError}</ErrorNotice> : null}
          </div>
          <SheetFooter className="flex-col-reverse border-t bg-muted/20 px-6 py-4 sm:flex-row sm:justify-end">
            <Button
              type="button"
              variant="outline"
              disabled={deleteMutation.isPending}
              onClick={() => setDeleteTarget(null)}
            >
                  {t("common.cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={!deleteTarget || deleteMutation.isPending}
              onClick={() => {
                if (deleteTarget) deleteMutation.mutate(deleteTarget)
              }}
            >
              {deleteMutation.isPending ? (
                <Loader2Icon data-icon="inline-start" className="animate-spin" />
              ) : (
                <Trash2Icon data-icon="inline-start" />
              )}
              {t("users.confirmDelete")}
            </Button>
          </SheetFooter>
        </SheetContent>
      </Sheet>
    </div>
  )
}

function UserTableRow({
  currentUserId,
  deletePending,
  onDelete,
  onReset,
  onToggleStatus,
  statusMutation,
  user,
}: {
  currentUserId: string | null
  deletePending: boolean
  onDelete: (user: AuthUser) => void
  onReset: (user: AuthUser) => void
  onToggleStatus: (user: AuthUser, status: AuthUserStatus) => void
  statusMutation: ReturnType<typeof useMutation<UserResponseLike, Error, StatusMutationVariables>>
  user: AuthUser
}) {
  const isSelf = currentUserId === user.id
  const nextStatus = user.status === "active" ? "disabled" : "active"
  const statusPending =
    statusMutation.isPending && statusMutation.variables?.user.id === user.id

  return (
    <TableRow className="group align-top">
      <TableCell className="pl-6">
        <UserIdentity isSelf={isSelf} user={user} />
      </TableCell>
      <TableCell>
        <RoleBadge role={user.role} />
      </TableCell>
      <TableCell>
        <StatusIndicator status={user.status} />
      </TableCell>
      <TableCell>
        <SecurityState mustChangePassword={user.must_change_password} />
      </TableCell>
      <TableCell className="font-mono text-xs text-muted-foreground">
        {formatDateTime(user.updated_at)}
      </TableCell>
      <TableCell className="pr-6">
        <UserActions
          disabled={isSelf}
          deletePending={deletePending}
          onDelete={() => onDelete(user)}
          onReset={() => onReset(user)}
          onToggle={() => onToggleStatus(user, nextStatus)}
          status={user.status}
          statusPending={statusPending}
        />
      </TableCell>
    </TableRow>
  )
}

function UserMobileCard({
  currentUserId,
  deletePending,
  onDelete,
  onReset,
  onToggleStatus,
  statusMutation,
  user,
}: {
  currentUserId: string | null
  deletePending: boolean
  onDelete: (user: AuthUser) => void
  onReset: (user: AuthUser) => void
  onToggleStatus: (user: AuthUser, status: AuthUserStatus) => void
  statusMutation: ReturnType<typeof useMutation<UserResponseLike, Error, StatusMutationVariables>>
  user: AuthUser
}) {
  const { t } = useI18n()
  const isSelf = currentUserId === user.id
  const nextStatus = user.status === "active" ? "disabled" : "active"
  const statusPending =
    statusMutation.isPending && statusMutation.variables?.user.id === user.id

  return (
    <article className="space-y-4 px-4 py-4">
      <div className="flex items-start justify-between gap-3">
        <UserIdentity isSelf={isSelf} user={user} />
        <RoleBadge role={user.role} />
      </div>
      <div className="grid gap-3 rounded-lg bg-muted/30 p-3 text-sm sm:grid-cols-3">
        <InfoLine label={t("users.headStatus")}>
          <StatusIndicator status={user.status} />
        </InfoLine>
        <InfoLine label={t("users.headSecurity")}>
          <SecurityState mustChangePassword={user.must_change_password} />
        </InfoLine>
        <InfoLine label={t("users.headUpdated")}>
          <span className="font-mono text-xs text-muted-foreground">
            {formatDateTime(user.updated_at)}
          </span>
        </InfoLine>
      </div>
      <UserActions
        disabled={isSelf}
        deletePending={deletePending}
        mobile
        onDelete={() => onDelete(user)}
        onReset={() => onReset(user)}
        onToggle={() => onToggleStatus(user, nextStatus)}
        status={user.status}
        statusPending={statusPending}
      />
    </article>
  )
}

type StatusMutationVariables = {
  user: AuthUser
  status: AuthUserStatus
}

type UserResponseLike = {
  user: AuthUser
}

function UserIdentity({ isSelf, user }: { isSelf: boolean; user: AuthUser }) {
  const { t } = useI18n()
  const displayName = user.display_name || user.username

  return (
    <div className="flex min-w-0 items-center gap-3">
      <Avatar className={userAvatarTone(displayName)} size="lg">
        <AvatarFallback className="bg-transparent font-semibold text-inherit">
          {userAvatarInitial(displayName)}
        </AvatarFallback>
      </Avatar>
      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium text-foreground">{displayName}</span>
          {isSelf ? (
            <Badge variant="outline" className="h-4 shrink-0 px-1.5 text-[10px]">
              {t("users.self")}
            </Badge>
          ) : null}
        </div>
        <div className="mt-0.5 flex min-w-0 items-center gap-1 truncate font-mono text-xs text-muted-foreground">
          <span className="truncate">{user.username}</span>
          {user.email ? (
            <>
              <span className="text-muted-foreground/50">·</span>
              <MailIcon className="size-3 shrink-0" />
              <span className="truncate">{user.email}</span>
            </>
          ) : null}
        </div>
      </div>
    </div>
  )
}

function RoleBadge({ role }: { role: string }) {
  const { t } = useI18n()
  if (role === "admin") {
    return (
      <Badge
        variant="outline"
        className="border-violet-500/20 bg-violet-500/10 text-violet-700"
      >
        {t("users.roleAdmin")}
      </Badge>
    )
  }
  return (
    <Badge variant="outline" className="bg-muted/60 text-muted-foreground">
      {t("users.roleOperator")}
    </Badge>
  )
}

function StatusIndicator({ status }: { status: string }) {
  const { t } = useI18n()
  const active = status === "active"
  return (
    <div className="flex items-center gap-1.5 text-sm">
      <span
        className={cn(
          "size-1.5 rounded-full",
          active ? "bg-emerald-500" : "bg-red-400"
        )}
      />
      <span className={active ? "text-foreground" : "text-muted-foreground"}>
        {statusLabel(status, t)}
      </span>
    </div>
  )
}

function SecurityState({
  mustChangePassword,
}: {
  mustChangePassword: boolean
}) {
  const { t } = useI18n()
  if (mustChangePassword) {
    return (
      <div className="flex w-fit items-center gap-1.5 rounded-md bg-amber-50 px-2 py-1 text-[11px] font-medium text-amber-700 ring-1 ring-amber-100">
        <ShieldAlertIcon className="size-3.5" />
        {t("users.passwordRequired")}
      </div>
    )
  }
  return (
    <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
      <ShieldCheckIcon className="size-3.5" />
      {t("users.normal")}
    </span>
  )
}

function UserActions({
  deletePending,
  disabled,
  mobile = false,
  onDelete,
  onReset,
  onToggle,
  status,
  statusPending,
}: {
  deletePending: boolean
  disabled: boolean
  mobile?: boolean
  onDelete: () => void
  onReset: () => void
  onToggle: () => void
  status: string
  statusPending: boolean
}) {
  const { t } = useI18n()
  const active = status === "active"
  const toggleLabel = active ? t("users.actionDisableUser") : t("users.actionEnableUser")
  return (
    <div
      className={cn(
        "flex items-center justify-end gap-1",
        mobile && "grid grid-cols-3 gap-2"
      )}
    >
      <Button
        type="button"
        variant="ghost"
        size={mobile ? "sm" : "icon-sm"}
        disabled={disabled}
        title={t("users.actionResetPassword")}
        onClick={onReset}
      >
        <KeyRoundIcon data-icon={mobile ? "inline-start" : undefined} />
        {mobile ? t("users.actionResetPassword") : <span className="sr-only">{t("users.actionResetPassword")}</span>}
      </Button>
      <Button
        type="button"
        variant="ghost"
        size={mobile ? "sm" : "icon-sm"}
        disabled={disabled || statusPending}
        title={toggleLabel}
        className={cn(
          active
            ? "text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            : "text-muted-foreground hover:bg-emerald-500/10 hover:text-emerald-700"
        )}
        onClick={onToggle}
      >
        {statusPending ? (
          <Loader2Icon
            data-icon={mobile ? "inline-start" : undefined}
            className="animate-spin"
          />
        ) : active ? (
          <UserXIcon data-icon={mobile ? "inline-start" : undefined} />
        ) : (
          <UserCheckIcon data-icon={mobile ? "inline-start" : undefined} />
        )}
        {mobile ? toggleLabel : <span className="sr-only">{toggleLabel}</span>}
      </Button>
      <Button
        type="button"
        variant="ghost"
        size={mobile ? "sm" : "icon-sm"}
        disabled={disabled || deletePending}
        title={t("users.actionDeleteUser")}
        className="text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
        onClick={onDelete}
      >
        {deletePending ? (
          <Loader2Icon
            data-icon={mobile ? "inline-start" : undefined}
            className="animate-spin"
          />
        ) : (
          <Trash2Icon data-icon={mobile ? "inline-start" : undefined} />
        )}
        {mobile ? t("common.delete") : <span className="sr-only">{t("users.actionDeleteUser")}</span>}
      </Button>
    </div>
  )
}

function InfoLine({
  children,
  label,
}: {
  children: React.ReactNode
  label: string
}) {
  return (
    <div className="space-y-1">
      <div className="text-xs text-muted-foreground">{label}</div>
      {children}
    </div>
  )
}

function LoadingRow() {
  const { t } = useI18n()
  return (
    <TableRow>
      <TableCell colSpan={6}>
        <div className="flex items-center gap-2 py-10 text-sm text-muted-foreground">
          <Loader2Icon className="size-4 animate-spin" />
          {t("users.loading")}
        </div>
      </TableCell>
    </TableRow>
  )
}

function EmptyRow({ search }: { search: string }) {
  return (
    <TableRow>
      <TableCell colSpan={6}>
        <EmptyState search={search} />
      </TableCell>
    </TableRow>
  )
}

function EmptyState({ search }: { search: string }) {
  const { t } = useI18n()
  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center text-muted-foreground">
      <div className="flex size-14 items-center justify-center rounded-full bg-muted">
        <InboxIcon className="size-7" />
      </div>
      <div className="mt-4 text-sm font-medium text-foreground">
        {search.trim() ? t("users.emptyFilteredTitle") : t("users.emptyTitle")}
      </div>
      <p className="mt-1 max-w-sm text-xs leading-5">
        {search.trim()
          ? t("users.emptyFilteredDescription")
          : t("users.emptyDescription")}
      </p>
    </div>
  )
}

function ErrorNotice({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
      <AlertCircleIcon className="mt-0.5 size-4 shrink-0" />
      <span>{children}</span>
    </div>
  )
}

function LabeledInput({
  autoComplete,
  disabled,
  id,
  label,
  onChange,
  placeholder,
  required = false,
  type = "text",
  value,
}: {
  autoComplete?: string
  disabled?: boolean
  id: string
  label: string
  onChange: (value: string) => void
  placeholder?: string
  required?: boolean
  type?: string
  value: string
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>
        {label}
        {required ? <span className="ml-1 text-destructive">*</span> : null}
      </Label>
      <Input
        id={id}
        type={type}
        autoComplete={autoComplete}
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  )
}

function roleLabel(role: string, t: (key: MessageKey) => string) {
  if (role === "admin") return t("users.roleAdmin")
  if (role === "operator") return t("users.roleOperator")
  return role
}

function statusLabel(status: string, t: (key: MessageKey) => string) {
  if (status === "active") return t("users.statusActive")
  if (status === "disabled") return t("users.statusDisabled")
  return status
}

function formatDateTime(timestamp: number) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return "-"
  return new Date(timestamp * 1000).toLocaleString()
}
