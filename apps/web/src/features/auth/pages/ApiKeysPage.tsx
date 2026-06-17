import { useMemo, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import {
  AlertCircleIcon,
  AlertTriangleIcon,
  BookOpenIcon,
  CheckCircle2Icon,
  CheckIcon,
  Code2Icon,
  ClipboardIcon,
  InboxIcon,
  KeyRoundIcon,
  Loader2Icon,
  PlusIcon,
  RefreshCwIcon,
  SearchIcon,
  ShieldOffIcon,
  TerminalSquareIcon,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
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
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { createApiKey, revokeApiKey } from "@/features/auth/api"
import { authQueryKeys, createApiKeysQueryOptions } from "@/features/auth/queries"
import type { ApiKeyRecord, ApiKeyScope, ApiKeyStatus } from "@/features/auth/types"
import { cn } from "@/lib/utils"

type ApiKeysPageProps = {
  baseUrl: string
}

type ScopeOption = {
  value: ApiKeyScope
  label: string
  description: string
}

type StatusFilter = ApiKeyStatus | "all"

const scopeOptions: ScopeOption[] = [
  {
    value: "template:read",
    label: "读取模板",
    description: "允许获取打印模板列表和详情。",
  },
  {
    value: "preview:create",
    label: "创建预览",
    description: "允许提交数据并生成文档预览。",
  },
  {
    value: "print:create",
    label: "创建打印任务",
    description: "允许提交真实的物理打印请求。",
  },
  {
    value: "printer:read",
    label: "读取打印机",
    description: "允许读取可用打印机和打印能力。",
  },
  {
    value: "job:read",
    label: "读取任务状态",
    description: "允许查询和轮询打印任务进度。",
  },
]

const defaultScopes: ApiKeyScope[] = [
  "template:read",
  "preview:create",
  "print:create",
  "printer:read",
  "job:read",
]

export function ApiKeysPage({ baseUrl }: ApiKeysPageProps) {
  const queryClient = useQueryClient()
  const [createOpen, setCreateOpen] = useState(false)
  const [docsOpen, setDocsOpen] = useState(false)
  const [name, setName] = useState("")
  const [scopes, setScopes] = useState<ApiKeyScope[]>(defaultScopes)
  const [createdToken, setCreatedToken] = useState("")
  const [copied, setCopied] = useState(false)
  const [search, setSearch] = useState("")
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("active")

  const apiKeysQueryOptions = useMemo(
    () => createApiKeysQueryOptions(baseUrl),
    [baseUrl]
  )
  const apiKeysQuery = useQuery(apiKeysQueryOptions)

  const createMutation = useMutation({
    mutationFn: () => createApiKey(baseUrl, { name, scopes }),
    onSuccess: async (response) => {
      setCreatedToken(response.token)
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.apiKeys(baseUrl),
      })
    },
  })

  const revokeMutation = useMutation({
    mutationFn: (apiKey: ApiKeyRecord) => revokeApiKey(baseUrl, apiKey.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: authQueryKeys.apiKeys(baseUrl),
      })
    },
  })

  const apiKeys = apiKeysQuery.data?.api_keys ?? []
  const filteredKeys = useMemo(() => {
    const keyword = search.trim().toLowerCase()
    return apiKeys.filter((apiKey) => {
      if (statusFilter !== "all" && apiKey.status !== statusFilter) {
        return false
      }
      if (!keyword) return true
      return [
        apiKey.name,
        apiKey.key_prefix,
        apiKey.status,
        statusLabel(apiKey.status),
        ...apiKey.scopes.map(scopeLabel),
        ...apiKey.scopes,
      ]
        .join(" ")
        .toLowerCase()
        .includes(keyword)
    })
  }, [apiKeys, search, statusFilter])

  const activeCount = apiKeys.filter((item) => item.status === "active").length
  const revokedCount = apiKeys.filter((item) => item.status === "revoked").length
  const createError =
    createMutation.error instanceof Error ? createMutation.error.message : null
  const revokeError =
    revokeMutation.error instanceof Error ? revokeMutation.error.message : null
  const canCreate =
    name.trim().length > 0 &&
    scopes.length > 0 &&
    !createMutation.isPending &&
    !createdToken

  const closeCreateSheet = () => {
    setCreateOpen(false)
    setName("")
    setScopes(defaultScopes)
    setCreatedToken("")
    setCopied(false)
    createMutation.reset()
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto bg-background px-3 py-4 sm:px-6 sm:py-6">
      <div className="mx-auto flex max-w-6xl flex-col gap-5">
        <header className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <div className="min-w-0">
            <h1 className="font-heading text-2xl font-semibold tracking-tight">
              API Key 管理
            </h1>
            <p className="mt-1 text-sm text-muted-foreground">
              管理外部系统调用开放 API 的身份凭证和权限范围。
            </p>
          </div>
          <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
            <StatusFilterSelect
              activeCount={activeCount}
              revokedCount={revokedCount}
              totalCount={apiKeys.length}
              value={statusFilter}
              onChange={setStatusFilter}
            />
            <div className="relative sm:w-72">
              <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                className="pl-8"
                placeholder="搜索名称、Prefix 或权限..."
              />
            </div>
            <Button
              type="button"
              variant="outline"
              onClick={() => setDocsOpen(true)}
            >
              <BookOpenIcon data-icon="inline-start" />
              接口文档
            </Button>
            <Button
              type="button"
              variant="outline"
              size="icon"
              title="刷新列表"
              onClick={() => void apiKeysQuery.refetch()}
              disabled={apiKeysQuery.isFetching}
            >
              <RefreshCwIcon
                className={apiKeysQuery.isFetching ? "animate-spin" : undefined}
              />
              <span className="sr-only">刷新列表</span>
            </Button>
            <Button
              type="button"
              onClick={() => {
                setCreatedToken("")
                setCopied(false)
                createMutation.reset()
                setCreateOpen(true)
              }}
            >
              <PlusIcon data-icon="inline-start" />
              新建 Key
            </Button>
          </div>
        </header>

        {apiKeysQuery.error ? (
          <ErrorNotice>
            {apiKeysQuery.error instanceof Error
              ? apiKeysQuery.error.message
              : "加载 API Key 失败"}
          </ErrorNotice>
        ) : null}
        {revokeError ? <ErrorNotice>{revokeError}</ErrorNotice> : null}

        <section className="overflow-hidden rounded-xl border bg-card shadow-sm">
          <div className="hidden md:block">
            <Table>
              <TableHeader className="bg-muted/40">
                <TableRow>
                  <TableHead className="pl-6">凭证名称</TableHead>
                  <TableHead className="w-52">安全前缀</TableHead>
                  <TableHead className="w-64">权限作用域</TableHead>
                  <TableHead className="w-28">状态</TableHead>
                  <TableHead className="w-44">最近调用</TableHead>
                  <TableHead className="w-24 pr-6 text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {apiKeysQuery.isPending ? (
                  <LoadingRow />
                ) : filteredKeys.length === 0 ? (
                  <EmptyRow search={search} statusFilter={statusFilter} />
                ) : (
                  filteredKeys.map((apiKey) => (
                    <ApiKeyTableRow
                      key={apiKey.id}
                      apiKey={apiKey}
                      onRevoke={() => revokeMutation.mutate(apiKey)}
                      revokePending={
                        revokeMutation.isPending &&
                        revokeMutation.variables?.id === apiKey.id
                      }
                    />
                  ))
                )}
              </TableBody>
            </Table>
          </div>

          <div className="divide-y md:hidden">
            {apiKeysQuery.isPending ? (
              Array.from({ length: 4 }).map((_, index) => (
                <div key={index} className="space-y-3 px-4 py-4">
                  <div className="h-4 w-2/3 animate-pulse rounded bg-muted" />
                  <div className="h-3 w-1/2 animate-pulse rounded bg-muted" />
                  <div className="h-8 w-full animate-pulse rounded bg-muted" />
                </div>
              ))
            ) : filteredKeys.length === 0 ? (
              <EmptyState search={search} statusFilter={statusFilter} />
            ) : (
              filteredKeys.map((apiKey) => (
                <ApiKeyMobileCard
                  key={apiKey.id}
                  apiKey={apiKey}
                  onRevoke={() => revokeMutation.mutate(apiKey)}
                  revokePending={
                    revokeMutation.isPending &&
                    revokeMutation.variables?.id === apiKey.id
                  }
                />
              ))
            )}
          </div>

          <div className="flex items-center justify-between border-t bg-muted/20 px-4 py-3 text-xs text-muted-foreground sm:px-6">
            <span>
              共 {filteredKeys.length} 个凭证
              {search.trim() || statusFilter !== "all"
                ? `，来自 ${apiKeys.length} 条记录`
                : ""}
            </span>
            <span>
              {activeCount} 个启用中，{revokedCount} 个已撤销
            </span>
          </div>
        </section>
      </div>

      <Sheet
        open={createOpen}
        onOpenChange={(open) => {
          if (open) {
            setCreateOpen(true)
          } else {
            closeCreateSheet()
          }
        }}
      >
        <SheetContent className="w-full max-w-none data-[side=right]:w-full data-[side=right]:sm:max-w-md">
          <SheetHeader className="border-b px-6 py-5">
            <SheetTitle>
              {createdToken ? "API Key 创建成功" : "新建 API Key"}
            </SheetTitle>
            <SheetDescription>
              {createdToken
                ? "请立即复制并妥善保管您的凭证。"
                : "按需分配权限，遵循最小权限原则。"}
            </SheetDescription>
          </SheetHeader>

          {createdToken ? (
            <ApiKeyCreatedState
              copied={copied}
              token={createdToken}
              onClose={closeCreateSheet}
              onCopy={() => void copyToken(createdToken, setCopied)}
            />
          ) : (
            <>
              <div className="flex flex-1 flex-col gap-6 overflow-y-auto px-6 py-5">
                <LabeledInput
                  id="api-key-name"
                  label="凭证名称"
                  required
                  value={name}
                  disabled={createMutation.isPending}
                  placeholder="例如：ERP 系统集成节点"
                  onChange={setName}
                />

                <div className="space-y-3">
                  <div className="flex items-center justify-between gap-3">
                    <Label>权限作用域</Label>
                    <span className="text-xs text-muted-foreground">
                      已选 {scopes.length} 项
                    </span>
                  </div>
                  <div className="space-y-2">
                    {scopeOptions.map((option) => (
                      <ScopeCheckItem
                        key={option.value}
                        disabled={createMutation.isPending}
                        option={option}
                        checked={scopes.includes(option.value)}
                        onCheckedChange={(checked) => {
                          setScopes((current) =>
                            checked
                              ? [...current, option.value]
                              : current.filter((scope) => scope !== option.value)
                          )
                        }}
                      />
                    ))}
                  </div>
                </div>

                {createError ? <ErrorNotice>{createError}</ErrorNotice> : null}
              </div>
              <SheetFooter className="flex-col-reverse border-t bg-muted/20 px-6 py-4 sm:flex-row sm:justify-end">
                <Button
                  type="button"
                  variant="outline"
                  disabled={createMutation.isPending}
                  onClick={closeCreateSheet}
                >
                  取消
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
                  生成凭证
                </Button>
              </SheetFooter>
            </>
          )}
        </SheetContent>
      </Sheet>

      <OpenApiDocsSheet
        baseUrl={baseUrl}
        open={docsOpen}
        onOpenChange={setDocsOpen}
      />
    </div>
  )
}

function StatusFilterSelect({
  activeCount,
  onChange,
  revokedCount,
  totalCount,
  value,
}: {
  activeCount: number
  onChange: (value: StatusFilter) => void
  revokedCount: number
  totalCount: number
  value: StatusFilter
}) {
  const options: { value: StatusFilter; label: string; count: number }[] = [
    { value: "active", label: "启用中", count: activeCount },
    { value: "all", label: "全部", count: totalCount },
    { value: "revoked", label: "已撤销", count: revokedCount },
  ]
  const selectedOption =
    options.find((option) => option.value === value) ?? options[0]

  return (
    <Select
      value={value}
      onValueChange={(nextValue) => onChange(nextValue as StatusFilter)}
    >
      <SelectTrigger className="w-full sm:w-36">
        <SelectValue>{selectedOption.label}</SelectValue>
      </SelectTrigger>
      <SelectContent align="start">
        <SelectGroup>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              <span>{option.label}</span>
              <span className="ml-auto text-xs text-muted-foreground">
                {option.count}
              </span>
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  )
}

type ApiDocEndpoint = {
  method: "GET" | "POST"
  path: string
  scope: string
  title: string
  description: string
  requestLabel?: string
  requestParams?: ApiDocParam[]
  responseParams?: ApiDocParam[]
  body?: string
  response?: string
}

type ApiDocParam = {
  name: string
  location?: string
  type: string
  required?: boolean
  description: string
  enumValues?: string
}

type PrintOptionDoc = {
  field: string
  type: string
  capability: string
  description: string
  examples: string
}

const jobStatusValues =
  "queued, rendering, submitting, printing, needs_attention, succeeded, failed, canceled"
const apiKeyStatusValues = "active, revoked"
const createJobResponseParams: ApiDocParam[] = [
  {
    name: "job_id",
    type: "string",
    required: true,
    description: "系统生成的任务 ID，用于按任务 ID 查询。",
  },
  {
    name: "status",
    type: "string",
    required: true,
    description: "任务当前状态。新任务通常为 queued。",
    enumValues: jobStatusValues,
  },
  {
    name: "idempotent",
    type: "boolean",
    required: true,
    description: "是否命中了相同 request_id 的已有任务。",
  },
]
const jobResponseParams: ApiDocParam[] = [
  {
    name: "job_id",
    type: "string",
    required: true,
    description: "系统任务 ID。",
  },
  {
    name: "request_id",
    type: "string",
    required: true,
    description: "调用方提交的业务幂等 ID。",
  },
  {
    name: "status",
    type: "string",
    required: true,
    description: "任务状态。",
    enumValues: jobStatusValues,
  },
  {
    name: "job_kind",
    type: "string",
    required: true,
    description: "任务来源类型。",
    enumValues: "template, direct_file",
  },
  {
    name: "printer_id",
    type: "string | null",
    description: "目标打印机 ID。",
  },
  {
    name: "printer_name_snapshot",
    type: "string | null",
    description: "提交任务时记录的打印机名称快照。",
  },
  {
    name: "last_error_code",
    type: "string | null",
    description: "失败或需介入时的错误码。",
  },
  {
    name: "last_error_message",
    type: "string | null",
    description: "失败或需介入时的错误说明。",
  },
  {
    name: "print_options",
    type: "object",
    description: "提交任务时实际保存的打印参数。",
  },
]

const printOptionDocs: PrintOptionDoc[] = [
  {
    field: "copies",
    type: "number",
    capability: "copies",
    description: "打印份数。必须在打印机声明的 min/max 范围内。",
    examples: "1, 2",
  },
  {
    field: "sides",
    type: "string",
    capability: "sides_supported",
    description: "单双面模式。只传当前打印机支持的枚举值。",
    examples: "one-sided, two-sided-long-edge, two-sided-short-edge",
  },
  {
    field: "printColorMode",
    type: "string",
    capability: "color_modes_supported / color_supported",
    description: "颜色模式。未声明能力时不要传；黑白通常是 monochrome。",
    examples: "color, monochrome",
  },
  {
    field: "media",
    type: "string",
    capability: "media_supported",
    description: "纸张大小。值直接使用能力接口返回的 CUPS/IPP keyword。",
    examples: "iso_a4_210x297mm, na_letter_8.5x11in",
  },
  {
    field: "mediaType",
    type: "string",
    capability: "media_types_supported",
    description: "纸张类型。未选择时可以不传，使用打印机默认。",
    examples: "stationery, photographic, photographic-glossy",
  },
  {
    field: "orientationRequested",
    type: "string",
    capability: "orientations_supported",
    description: "页面方向。后端会转换成 IPP 的 orientation-requested。",
    examples: "portrait, landscape",
  },
  {
    field: "printScaling",
    type: "string",
    capability: "scalings_supported",
    description: "缩放方式，主要影响直接文件/图片转 PDF 的页面适配。",
    examples: "auto, auto-fit, fit, fill, none",
  },
  {
    field: "pageRanges",
    type: "string",
    capability: "supports_page_ranges",
    description: "页码范围。只有 supports_page_ranges 为 true 时才传。",
    examples: "1-3 5 7-9",
  },
]

const scopeDocs = [
  {
    scope: "template:read",
    description: "读取模板列表。",
    endpoints: "GET /v1/open/templates",
  },
  {
    scope: "printer:read",
    description: "读取打印机列表和能力。",
    endpoints: "GET /v1/open/printers, GET /v1/open/printers/{printer_id}",
  },
  {
    scope: "preview:create",
    description: "创建 PDF 预览。",
    endpoints: "POST /v1/open/preview",
  },
  {
    scope: "print:create",
    description: "创建模板打印或文件打印任务。",
    endpoints: "POST /v1/open/print, POST /v1/open/print/direct",
  },
  {
    scope: "job:read",
    description: "查询任务状态。",
    endpoints: "GET /v1/open/jobs/{job_id}, GET /v1/open/jobs/by-request-id/{request_id}",
  },
]

const apiDocEndpoints: ApiDocEndpoint[] = [
  {
    method: "GET",
    path: "/v1/open/me",
    scope: "任意有效 API Key",
    title: "测试连接",
    description: "返回当前 API Key 的名称、Prefix、权限范围和过期时间。",
    responseParams: [
      {
        name: "api_key.id",
        type: "string",
        required: true,
        description: "API Key 记录 ID。",
      },
      {
        name: "api_key.name",
        type: "string",
        required: true,
        description: "创建 API Key 时填写的名称。",
      },
      {
        name: "api_key.key_prefix",
        type: "string",
        required: true,
        description: "安全前缀，可用于后台排查，不是完整密钥。",
      },
      {
        name: "api_key.scopes",
        type: "string[]",
        required: true,
        description: "当前 Key 拥有的权限作用域。",
        enumValues:
          "template:read, printer:read, preview:create, print:create, job:read",
      },
      {
        name: "api_key.status",
        type: "string",
        required: true,
        description: "凭证状态。",
        enumValues: apiKeyStatusValues,
      },
      {
        name: "api_key.expires_at",
        type: "number | null",
        description: "过期时间，Unix 秒；null 表示未设置过期时间。",
      },
    ],
    response: `{
  "api_key": {
    "id": "api-key-xxx",
    "name": "ERP Integration",
    "key_prefix": "abc123def456",
    "scopes": ["template:read", "printer:read", "print:create"],
    "status": "active",
    "created_at": 1710000000,
    "updated_at": 1710000000,
    "last_used_at": 1710000300,
    "expires_at": null
  }
}`,
  },
  {
    method: "GET",
    path: "/v1/open/templates",
    scope: "template:read",
    title: "获取模板列表",
    description: "返回模板分组和模板信息，用于选择 template_id。",
    responseParams: [
      {
        name: "groups",
        type: "array",
        required: true,
        description: "模板分组列表。",
      },
      {
        name: "groups[].id",
        type: "string",
        required: true,
        description: "模板分组 ID。",
      },
      {
        name: "groups[].templates[].id",
        type: "string",
        required: true,
        description: "模板 ID，用于 preview/print 的 template_id。",
      },
      {
        name: "groups[].templates[].name",
        type: "string",
        required: true,
        description: "模板名称。",
      },
      {
        name: "groups[].templates[].sample_data",
        type: "string",
        description: "模板示例数据 JSON 字符串，可帮助调用方理解 data 结构。",
      },
    ],
    response: `{
  "groups": [
    {
      "id": "group-xxx",
      "name": "发货单",
      "templates": [
        {
          "id": "template-xxx",
          "name": "标准发货单",
          "description": "A4 发货单模板",
          "output_name": "delivery-note.pdf"
        }
      ]
    }
  ]
}`,
  },
  {
    method: "GET",
    path: "/v1/open/printers",
    scope: "printer:read",
    title: "获取打印机列表",
    description: "返回已启用/已管理的打印机，用于选择 printer_id。",
    responseParams: [
      {
        name: "printers",
        type: "array",
        required: true,
        description: "已管理打印机列表。",
      },
      {
        name: "printers[].id",
        type: "string",
        required: true,
        description: "打印机 ID，用于 print/direct 的 printer_id。",
      },
      {
        name: "printers[].name",
        type: "string",
        required: true,
        description: "打印机显示名称。",
      },
      {
        name: "printers[].enabled",
        type: "boolean",
        required: true,
        description: "是否启用。未启用的打印机不能提交任务。",
      },
      {
        name: "printers[].state",
        type: "string | null",
        description: "CUPS/IPP 返回的最近状态。",
        enumValues: "idle, processing, stopped, unknown/null",
      },
    ],
    response: `{
  "printers": [
    {
      "id": "printer-xxx",
      "name": "Office Printer",
      "uri": "ipp://cups.local/printers/Office",
      "is_default": true,
      "enabled": true,
      "state": "idle"
    }
  ]
}`,
  },
  {
    method: "GET",
    path: "/v1/open/printers/{printer_id}",
    scope: "printer:read",
    title: "获取打印机能力",
    description: "返回 CUPS/IPP 声明的纸张、单双面、份数、颜色等能力；print_options 应按这里返回的能力取值。",
    requestParams: [
      {
        name: "printer_id",
        location: "path",
        type: "string",
        required: true,
        description: "打印机 ID，来自 /v1/open/printers。",
      },
    ],
    responseParams: [
      {
        name: "id",
        type: "string",
        required: true,
        description: "打印机 ID。",
      },
      {
        name: "enabled",
        type: "boolean",
        required: true,
        description: "是否启用。",
      },
      {
        name: "capabilities.media_supported",
        type: "string[]",
        description: "支持的纸张尺寸，可作为 print_options.media。",
      },
      {
        name: "capabilities.sides_supported",
        type: "string[]",
        description: "支持的单双面选项，可作为 print_options.sides。",
        enumValues: "one-sided, two-sided-long-edge, two-sided-short-edge",
      },
      {
        name: "capabilities.color_modes_supported",
        type: "string[]",
        description: "支持的颜色模式，可作为 print_options.printColorMode。",
        enumValues: "color, monochrome",
      },
      {
        name: "capabilities.copies",
        type: "object | null",
        description: "份数能力，包含 default/min/max。",
      },
      {
        name: "capabilities.supports_page_ranges",
        type: "boolean | null",
        description: "是否支持 pageRanges。null 表示打印机未声明。",
      },
    ],
    response: `{
  "id": "printer-xxx",
  "name": "Office Printer",
  "capabilities": {
    "media_supported": ["iso_a4_210x297mm"],
    "media_default": "iso_a4_210x297mm",
    "media_types_supported": ["stationery"],
    "sides_supported": ["one-sided", "two-sided-long-edge"],
    "copies": { "default": 1, "min": 1, "max": 99 },
    "color_modes_supported": ["color", "monochrome"],
    "orientations_supported": ["portrait", "landscape"],
    "scalings_supported": ["fit", "fill"],
    "supports_page_ranges": true,
    "job_creation_attributes_supported": ["copies", "media", "sides"]
  }
}`,
  },
  {
    method: "POST",
    path: "/v1/open/preview",
    scope: "preview:create",
    title: "生成模板预览",
    description: "传入模板 ID 和 JSON 数据，返回 PDF 预览文件；纸张等 print_options 会参与预览渲染。",
    requestParams: [
      {
        name: "template_id",
        location: "body",
        type: "string",
        required: true,
        description: "模板 ID，来自 /v1/open/templates。",
      },
      {
        name: "data",
        location: "body",
        type: "object",
        required: true,
        description: "模板数据 JSON。字段结构由模板约定。",
      },
      {
        name: "print_options",
        location: "body",
        type: "object",
        description: "打印/预览参数。字段见上方 print_options 表。",
      },
    ],
    responseParams: [
      {
        name: "Content-Type",
        location: "header",
        type: "application/pdf",
        required: true,
        description: "响应体为 PDF 二进制。",
      },
      {
        name: "x-deepprint-preview-page-count",
        location: "header",
        type: "number",
        description: "预览页数。",
      },
      {
        name: "x-deepprint-preview-page-width-pt",
        location: "header",
        type: "number",
        description: "页面宽度，单位 pt。",
      },
    ],
    response: `HTTP 200
Content-Type: application/pdf
x-deepprint-preview-output-kind: pdf
x-deepprint-preview-page-count: 1

<PDF bytes>`,
    body: `{
  "template_id": "template-xxx",
  "data": { "orderNo": "A1001" },
  "print_options": { "media": "iso_a4_210x297mm" }
}`,
  },
  {
    method: "POST",
    path: "/v1/open/print",
    scope: "print:create",
    title: "创建模板打印任务",
    description: "按模板和数据创建真实打印任务，request_id 用于幂等提交。",
    requestParams: [
      {
        name: "request_id",
        location: "body",
        type: "string",
        required: true,
        description: "调用方业务幂等 ID。相同 request_id 会返回已有任务。",
      },
      {
        name: "template_id",
        location: "body",
        type: "string",
        required: true,
        description: "模板 ID。",
      },
      {
        name: "printer_id",
        location: "body",
        type: "string",
        required: true,
        description: "目标打印机 ID。",
      },
      {
        name: "data",
        location: "body",
        type: "object",
        required: true,
        description: "模板数据 JSON。",
      },
      {
        name: "print_options",
        location: "body",
        type: "object",
        description: "打印参数。只提交当前打印机支持的字段和值。",
      },
    ],
    responseParams: createJobResponseParams,
    response: `{
  "job_id": "job-xxx",
  "status": "queued",
  "idempotent": false
}`,
    body: `{
  "request_id": "erp-order-A1001",
  "template_id": "template-xxx",
  "printer_id": "printer-xxx",
  "data": { "orderNo": "A1001" },
  "print_options": {
    "copies": 1,
    "media": "iso_a4_210x297mm",
    "sides": "one-sided",
    "printColorMode": "monochrome"
  }
}`,
  },
  {
    method: "POST",
    path: "/v1/open/print/direct",
    scope: "print:create",
    title: "创建文件打印任务",
    description: "使用 multipart/form-data 直接上传 PDF/图片等文件内容，避免 base64 膨胀。",
    requestLabel: "multipart/form-data 字段",
    requestParams: [
      {
        name: "request_id",
        location: "form",
        type: "string",
        required: true,
        description: "调用方业务幂等 ID。相同 request_id 会返回已有任务。",
      },
      {
        name: "printer_id",
        location: "form",
        type: "string",
        required: true,
        description: "目标打印机 ID。",
      },
      {
        name: "file",
        location: "form",
        type: "file",
        required: true,
        description: "要打印的 PDF/图片等文件。文件大小受服务端 direct_job_max_bytes 限制。",
      },
      {
        name: "print_options",
        location: "form",
        type: "JSON string",
        description: "打印参数 JSON 字符串，例如 {\"copies\":1}。",
      },
      {
        name: "content_type",
        location: "form",
        type: "string",
        description: "可选。通常可由 file part 的 Content-Type 提供。",
      },
    ],
    responseParams: createJobResponseParams,
    body: `request_id=erp-file-A1001
printer_id=printer-xxx
file=@invoice.pdf
print_options={"copies":1,"media":"iso_a4_210x297mm","printScaling":"fit"}

curl -X POST "${"${baseUrl}"}/v1/open/print/direct" \\
  -H "Authorization: Bearer YOUR_API_KEY" \\
  -F "request_id=erp-file-A1001" \\
  -F "printer_id=printer-xxx" \\
  -F 'print_options={"copies":1,"media":"iso_a4_210x297mm","printScaling":"fit"}' \\
  -F "file=@invoice.pdf;type=application/pdf"`,
    response: `{
  "job_id": "job-xxx",
  "status": "queued",
  "idempotent": false
}`,
  },
  {
    method: "GET",
    path: "/v1/open/jobs/{job_id}",
    scope: "job:read",
    title: "按任务 ID 查询",
    description: "按创建任务返回的 job_id 查询打印任务状态。",
    requestParams: [
      {
        name: "job_id",
        location: "path",
        type: "string",
        required: true,
        description: "创建任务响应返回的 job_id。",
      },
    ],
    responseParams: jobResponseParams,
    response: `{
  "job_id": "job-xxx",
  "request_id": "erp-order-A1001",
  "status": "printing",
  "printer_id": "printer-xxx",
  "printer_name_snapshot": "Office Printer",
  "last_error_code": null,
  "last_error_message": null,
  "print_options": { "copies": 1 }
}`,
  },
  {
    method: "GET",
    path: "/v1/open/jobs/by-request-id/{request_id}",
    scope: "job:read",
    title: "按业务请求 ID 查询",
    description: "当提交响应丢失时，可用 request_id 恢复查询同一个任务。",
    requestParams: [
      {
        name: "request_id",
        location: "path",
        type: "string",
        required: true,
        description: "调用方提交任务时使用的 request_id。",
      },
    ],
    responseParams: jobResponseParams,
    response: `{
  "job_id": "job-xxx",
  "request_id": "erp-order-A1001",
  "status": "queued",
  "printer_id": "printer-xxx",
  "printer_name_snapshot": "Office Printer",
  "last_error_code": null,
  "last_error_message": null,
  "print_options": { "copies": 1 }
}`,
  },
]

function OpenApiDocsSheet({
  baseUrl,
  onOpenChange,
  open,
}: {
  baseUrl: string
  onOpenChange: (open: boolean) => void
  open: boolean
}) {
  const normalizedBaseUrl = baseUrl.replace(/\/$/, "")

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full max-w-none data-[side=right]:w-full data-[side=right]:sm:max-w-2xl">
        <SheetHeader className="border-b px-6 py-5">
          <SheetTitle>开放接口文档</SheetTitle>
          <SheetDescription>
            外部系统使用 API Key 通过 Bearer Token 调用这些接口。
          </SheetDescription>
        </SheetHeader>

        <div className="flex-1 overflow-y-auto px-6 py-5">
          <section className="rounded-xl border bg-muted/20 p-4">
            <div className="flex items-start gap-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Code2Icon className="size-4" />
              </div>
              <div className="min-w-0 space-y-2">
                <div className="font-medium">认证方式</div>
                <p className="text-sm leading-6 text-muted-foreground">
                  请求头统一携带{" "}
                  <code className="rounded bg-background px-1.5 py-0.5 font-mono text-xs">
                    Authorization: Bearer YOUR_API_KEY
                  </code>
                  。完整 Token 只会在创建时显示一次。
                </p>
                <code className="block overflow-x-auto rounded-lg bg-slate-950 px-3 py-2 font-mono text-xs leading-5 text-emerald-300">
                  {`curl -H "Authorization: Bearer dp_xxx" ${normalizedBaseUrl}/v1/open/me`}
                </code>
              </div>
            </div>
          </section>

          <section className="mt-4 overflow-hidden rounded-xl border bg-card">
            <div className="border-b bg-muted/30 px-4 py-3">
              <div className="font-medium">权限作用域与接口</div>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">
                创建 API Key 时只选择实际需要的 scope。当前开放接口只使用下列 5 个作用域。
              </p>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[760px] text-left text-sm">
                <thead className="bg-muted/20 text-xs text-muted-foreground">
                  <tr>
                    <th className="px-4 py-2 font-medium">Scope</th>
                    <th className="px-4 py-2 font-medium">说明</th>
                    <th className="px-4 py-2 font-medium">对应接口</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {scopeDocs.map((scope) => (
                    <tr key={scope.scope}>
                      <td className="px-4 py-3 align-top">
                        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                          {scope.scope}
                        </code>
                      </td>
                      <td className="px-4 py-3 align-top text-muted-foreground">
                        {scope.description}
                      </td>
                      <td className="px-4 py-3 align-top font-mono text-xs text-muted-foreground">
                        {scope.endpoints}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <section className="mt-4 rounded-xl border bg-card p-4">
            <div className="flex items-start gap-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-amber-500/10 text-amber-600">
                <AlertCircleIcon className="size-4" />
              </div>
              <div className="min-w-0 space-y-2">
                <div className="font-medium">print_options 怎么填</div>
                <p className="text-sm leading-6 text-muted-foreground">
                  先调用{" "}
                  <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                    GET /v1/open/printers/{"{printer_id}"}
                  </code>{" "}
                  获取当前打印机能力，再只提交该打印机声明支持的字段和值。后端会校验能力；未声明或不支持的值会返回错误，不会静默忽略。
                </p>
                <pre className="overflow-x-auto rounded-lg bg-slate-950 px-3 py-3 text-xs leading-5 text-slate-100">
                  <code>{`{
  "print_options": {
    "copies": 1,
    "media": "iso_a4_210x297mm",
    "sides": "one-sided",
    "printColorMode": "monochrome",
    "orientationRequested": "portrait",
    "printScaling": "fit",
    "pageRanges": "1-3 5"
  }
}`}</code>
                </pre>
              </div>
            </div>
          </section>

          <section className="mt-4 overflow-hidden rounded-xl border bg-card">
            <div className="border-b bg-muted/30 px-4 py-3">
              <div className="font-medium">print_options 字段与能力映射</div>
              <p className="mt-1 text-xs leading-5 text-muted-foreground">
                字段名使用 JSON 驼峰；能力字段来自打印机详情接口的{" "}
                <code className="rounded bg-background px-1 py-0.5 font-mono">
                  capabilities
                </code>
                。
              </p>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[720px] text-left text-sm">
                <thead className="bg-muted/20 text-xs text-muted-foreground">
                  <tr>
                    <th className="px-4 py-2 font-medium">字段</th>
                    <th className="px-4 py-2 font-medium">类型</th>
                    <th className="px-4 py-2 font-medium">看哪个能力</th>
                    <th className="px-4 py-2 font-medium">说明</th>
                    <th className="px-4 py-2 font-medium">示例值</th>
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {printOptionDocs.map((option) => (
                    <tr key={option.field}>
                      <td className="px-4 py-3 align-top">
                        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
                          {option.field}
                        </code>
                      </td>
                      <td className="px-4 py-3 align-top font-mono text-xs text-muted-foreground">
                        {option.type}
                      </td>
                      <td className="px-4 py-3 align-top font-mono text-xs text-muted-foreground">
                        {option.capability}
                      </td>
                      <td className="px-4 py-3 align-top text-muted-foreground">
                        {option.description}
                      </td>
                      <td className="px-4 py-3 align-top font-mono text-xs text-muted-foreground">
                        {option.examples}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <div className="mt-5 space-y-3">
            {apiDocEndpoints.map((endpoint) => (
              <ApiDocEndpointCard
                key={`${endpoint.method}-${endpoint.path}`}
                endpoint={endpoint}
                baseUrl={normalizedBaseUrl}
              />
            ))}
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}

function ApiDocEndpointCard({
  baseUrl,
  endpoint,
}: {
  baseUrl: string
  endpoint: ApiDocEndpoint
}) {
  const renderedBody = endpoint.body?.replaceAll("${baseUrl}", baseUrl)
  return (
    <article className="overflow-hidden rounded-xl border bg-card">
      <div className="flex flex-col gap-3 border-b bg-muted/30 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              variant={endpoint.method === "GET" ? "secondary" : "default"}
              className="font-mono"
            >
              {endpoint.method}
            </Badge>
            <span className="font-medium">{endpoint.title}</span>
          </div>
          <code className="mt-1 block truncate font-mono text-xs text-muted-foreground">
            {endpoint.path}
          </code>
        </div>
        <Badge variant="outline" className="w-fit bg-background font-mono text-[11px]">
          {endpoint.scope}
        </Badge>
      </div>
      <div className="space-y-3 px-4 py-3">
        <p className="text-sm leading-6 text-muted-foreground">
          {endpoint.description}
        </p>
        <code className="block overflow-x-auto rounded-lg bg-muted px-3 py-2 font-mono text-xs text-muted-foreground">
          {endpoint.method} {baseUrl}
          {endpoint.path}
        </code>
        {endpoint.requestParams?.length ? (
          <ApiDocParamsTable title="请求参数" params={endpoint.requestParams} />
        ) : null}
        {renderedBody ? (
          <div className="space-y-2">
            <div className="text-xs font-medium text-muted-foreground">
              {endpoint.requestLabel ?? "请求示例"}
            </div>
            <pre className="overflow-x-auto rounded-lg bg-slate-950 px-3 py-3 text-xs leading-5 text-slate-100">
              <code>{renderedBody}</code>
            </pre>
          </div>
        ) : null}
        {endpoint.responseParams?.length ? (
          <ApiDocParamsTable title="响应参数" params={endpoint.responseParams} />
        ) : null}
        {endpoint.response ? (
          <div className="space-y-2">
            <div className="text-xs font-medium text-muted-foreground">响应示例</div>
            <pre className="overflow-x-auto rounded-lg bg-slate-950 px-3 py-3 text-xs leading-5 text-slate-100">
              <code>{endpoint.response}</code>
            </pre>
          </div>
        ) : null}
      </div>
    </article>
  )
}

function ApiDocParamsTable({
  params,
  title,
}: {
  params: ApiDocParam[]
  title: string
}) {
  return (
    <div className="space-y-2">
      <div className="text-xs font-medium text-muted-foreground">{title}</div>
      <div className="overflow-x-auto rounded-lg border">
        <table className="w-full min-w-[720px] text-left text-xs">
          <thead className="bg-muted/30 text-muted-foreground">
            <tr>
              <th className="px-3 py-2 font-medium">名称</th>
              <th className="px-3 py-2 font-medium">位置</th>
              <th className="px-3 py-2 font-medium">类型</th>
              <th className="px-3 py-2 font-medium">必填</th>
              <th className="px-3 py-2 font-medium">说明</th>
              <th className="px-3 py-2 font-medium">枚举/取值</th>
            </tr>
          </thead>
          <tbody className="divide-y">
            {params.map((param) => (
              <tr key={`${title}-${param.name}-${param.location ?? "body"}`}>
                <td className="px-3 py-2 align-top">
                  <code className="rounded bg-muted px-1.5 py-0.5 font-mono">
                    {param.name}
                  </code>
                </td>
                <td className="px-3 py-2 align-top font-mono text-muted-foreground">
                  {param.location ?? "-"}
                </td>
                <td className="px-3 py-2 align-top font-mono text-muted-foreground">
                  {param.type}
                </td>
                <td className="px-3 py-2 align-top text-muted-foreground">
                  {param.required ? "是" : "否"}
                </td>
                <td className="px-3 py-2 align-top text-muted-foreground">
                  {param.description}
                </td>
                <td className="px-3 py-2 align-top font-mono text-muted-foreground">
                  {param.enumValues ?? "-"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function ApiKeyTableRow({
  apiKey,
  onRevoke,
  revokePending,
}: {
  apiKey: ApiKeyRecord
  onRevoke: () => void
  revokePending: boolean
}) {
  return (
    <TableRow className="group align-top">
      <TableCell className="pl-6">
        <ApiKeyIdentity apiKey={apiKey} />
      </TableCell>
      <TableCell>
        <MaskedPrefix prefix={apiKey.key_prefix} />
      </TableCell>
      <TableCell>
        <ScopeBadges scopes={apiKey.scopes} compact />
      </TableCell>
      <TableCell>
        <StatusIndicator status={apiKey.status} />
      </TableCell>
      <TableCell className="font-mono text-xs text-muted-foreground">
        {apiKey.last_used_at ? (
          formatDateTime(apiKey.last_used_at)
        ) : (
          <span className="font-sans italic text-muted-foreground/80">
            从未调用
          </span>
        )}
      </TableCell>
      <TableCell className="pr-6">
        <div className="flex justify-end">
          <RevokeButton
            disabled={apiKey.status !== "active"}
            pending={revokePending}
            onClick={onRevoke}
          />
        </div>
      </TableCell>
    </TableRow>
  )
}

function ApiKeyMobileCard({
  apiKey,
  onRevoke,
  revokePending,
}: {
  apiKey: ApiKeyRecord
  onRevoke: () => void
  revokePending: boolean
}) {
  return (
    <article className="space-y-4 px-4 py-4">
      <div className="flex items-start justify-between gap-3">
        <ApiKeyIdentity apiKey={apiKey} />
        <StatusIndicator status={apiKey.status} />
      </div>
      <div className="space-y-3 rounded-lg bg-muted/30 p-3">
        <InfoLine label="安全前缀">
          <MaskedPrefix prefix={apiKey.key_prefix} />
        </InfoLine>
        <InfoLine label="权限作用域">
          <ScopeBadges scopes={apiKey.scopes} />
        </InfoLine>
        <InfoLine label="最近调用">
          <span className="font-mono text-xs text-muted-foreground">
            {apiKey.last_used_at ? formatDateTime(apiKey.last_used_at) : "从未调用"}
          </span>
        </InfoLine>
      </div>
      <RevokeButton
        mobile
        disabled={apiKey.status !== "active"}
        pending={revokePending}
        onClick={onRevoke}
      />
    </article>
  )
}

function ApiKeyIdentity({ apiKey }: { apiKey: ApiKeyRecord }) {
  const active = apiKey.status === "active"
  return (
    <div className="flex min-w-0 items-center gap-3">
      <div
        className={cn(
          "flex size-9 shrink-0 items-center justify-center rounded-lg",
          active
            ? "bg-indigo-500/10 text-indigo-600"
            : "bg-muted text-muted-foreground"
        )}
      >
        <KeyRoundIcon className="size-4" />
      </div>
      <div className="min-w-0">
        <div className="truncate font-medium text-foreground" title={apiKey.name}>
          {apiKey.name}
        </div>
        <div className="mt-0.5 text-xs text-muted-foreground">
          创建于 {formatDateOnly(apiKey.created_at)}
        </div>
      </div>
    </div>
  )
}

function MaskedPrefix({ prefix }: { prefix: string }) {
  return (
    <code className="inline-flex max-w-full items-center rounded-md border bg-muted/60 px-2 py-1 font-mono text-xs text-muted-foreground">
      <span className="truncate">{prefix}</span>
      <span className="ml-1 tracking-[0.2em] text-muted-foreground/60">
        ********
      </span>
    </code>
  )
}

function ScopeBadges({
  compact = false,
  scopes,
}: {
  compact?: boolean
  scopes: string[]
}) {
  const visibleScopes = compact ? scopes.slice(0, 2) : scopes
  const hiddenScopes = compact ? scopes.slice(visibleScopes.length) : []

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {visibleScopes.map((scope) => (
        <Badge key={scope} variant="outline" className="bg-muted/60">
          {scopeLabel(scope)}
        </Badge>
      ))}
      {hiddenScopes.length ? (
        <Tooltip>
          <TooltipTrigger
            render={
              <span className="inline-flex cursor-help items-center justify-center rounded-full border bg-muted/60 px-2 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted" />
            }
          >
            +{hiddenScopes.length}
          </TooltipTrigger>
          <TooltipContent side="top" align="center" className="block max-w-72 px-3 py-2">
            <div className="mb-1 border-b border-background/20 pb-1 font-medium text-background/80">
              全部权限作用域
            </div>
            <div className="space-y-1">
              {scopes.map((scope) => (
                <div key={scope} className="flex items-center gap-1.5 whitespace-nowrap">
                  <KeyRoundIcon className="size-3 text-background/70" />
                  <span>{scopeLabel(scope)}</span>
                  <span className="font-mono text-[11px] text-background/60">
                    {scope}
                  </span>
                </div>
              ))}
            </div>
          </TooltipContent>
        </Tooltip>
      ) : null}
    </div>
  )
}

function StatusIndicator({ status }: { status: string }) {
  const active = status === "active"
  return (
    <div className="flex items-center gap-1.5 text-sm">
      <span
        className={cn(
          "size-1.5 rounded-full",
          active ? "bg-emerald-500" : "bg-slate-300"
        )}
      />
      <span className={active ? "text-foreground" : "text-muted-foreground"}>
        {statusLabel(status)}
      </span>
    </div>
  )
}

function RevokeButton({
  disabled,
  mobile = false,
  onClick,
  pending,
}: {
  disabled: boolean
  mobile?: boolean
  onClick: () => void
  pending: boolean
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size={mobile ? "sm" : "icon-sm"}
      disabled={disabled || pending}
      title={disabled ? "该凭证已撤销" : "撤销凭证"}
      className={cn(
        "text-muted-foreground hover:bg-destructive/10 hover:text-destructive",
        mobile && "w-full"
      )}
      onClick={onClick}
    >
      {pending ? (
        <Loader2Icon
          data-icon={mobile ? "inline-start" : undefined}
          className="animate-spin"
        />
      ) : (
        <ShieldOffIcon data-icon={mobile ? "inline-start" : undefined} />
      )}
      {mobile ? "撤销凭证" : <span className="sr-only">撤销凭证</span>}
    </Button>
  )
}

function ScopeCheckItem({
  checked,
  disabled,
  onCheckedChange,
  option,
}: {
  checked: boolean
  disabled: boolean
  onCheckedChange: (checked: boolean) => void
  option: ScopeOption
}) {
  return (
    <label
      className={cn(
        "flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-colors",
        checked
          ? "border-primary/30 bg-primary/5"
          : "border-border bg-background hover:border-foreground/20",
        disabled && "cursor-not-allowed opacity-60"
      )}
    >
      <Checkbox
        checked={checked}
        disabled={disabled}
        className="mt-0.5"
        onCheckedChange={(value) => onCheckedChange(value === true)}
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-sm font-medium text-foreground">
            {option.label}
          </span>
          <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            {option.value}
          </code>
        </div>
        <p
          className="mt-1 text-xs leading-5 text-muted-foreground"
        >
          {option.description}
        </p>
      </div>
    </label>
  )
}

function ApiKeyCreatedState({
  copied,
  onClose,
  onCopy,
  token,
}: {
  copied: boolean
  onClose: () => void
  onCopy: () => void
  token: string
}) {
  return (
    <>
      <div className="flex flex-1 flex-col overflow-y-auto px-6 py-8">
        <div className="flex flex-1 flex-col items-center justify-center text-center">
          <div className="mb-6 flex size-16 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-600">
            <CheckCircle2Icon className="size-8" />
          </div>
          <h3 className="font-heading text-xl font-semibold tracking-tight">
            API Key 已生成
          </h3>
          <p className="mt-2 max-w-xs text-sm leading-6 text-muted-foreground">
            出于安全考虑，完整凭证仅在此刻显示一次。离开此页面后将无法再次查看。
          </p>

          <div className="mt-8 w-full rounded-xl bg-slate-950 p-4 text-left shadow-sm">
            <code className="break-all font-mono text-sm leading-relaxed text-emerald-300">
              {token}
            </code>
          </div>

          <Button
            type="button"
            variant={copied ? "outline" : "secondary"}
            className={cn(
              "mt-4 w-full",
              copied && "border-emerald-200 bg-emerald-50 text-emerald-700"
            )}
            onClick={onCopy}
          >
            {copied ? (
              <CheckIcon data-icon="inline-start" />
            ) : (
              <ClipboardIcon data-icon="inline-start" />
            )}
            {copied ? "已复制到剪贴板" : "复制完整 Token"}
          </Button>

          <div className="mt-6 flex items-start gap-2 rounded-lg border border-amber-100 bg-amber-50/70 p-3 text-left text-xs leading-5 text-amber-800">
            <AlertTriangleIcon className="mt-0.5 size-4 shrink-0 text-amber-500" />
            <p>
              请将 Token 保存在安全的环境变量或密钥管理服务中，切勿硬编码在前端代码或公开仓库里。
            </p>
          </div>
        </div>
      </div>
      <SheetFooter className="border-t bg-muted/20 px-6 py-4">
        <Button type="button" className="w-full" onClick={onClose}>
          我已妥善保存，关闭窗口
        </Button>
      </SheetFooter>
    </>
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
  return (
    <TableRow>
      <TableCell colSpan={6}>
        <div className="flex items-center gap-2 py-10 text-sm text-muted-foreground">
          <Loader2Icon className="size-4 animate-spin" />
          正在加载 API Key
        </div>
      </TableCell>
    </TableRow>
  )
}

function EmptyRow({
  search,
  statusFilter,
}: {
  search: string
  statusFilter: StatusFilter
}) {
  return (
    <TableRow>
      <TableCell colSpan={6}>
        <EmptyState search={search} statusFilter={statusFilter} />
      </TableCell>
    </TableRow>
  )
}

function EmptyState({
  search,
  statusFilter,
}: {
  search: string
  statusFilter: StatusFilter
}) {
  const searching = search.trim().length > 0
  const filteredByStatus = statusFilter !== "all"
  const title = searching
    ? "没有找到符合条件的 API Key"
    : filteredByStatus
      ? `暂无${statusLabel(statusFilter)}凭证`
      : "暂无 API Key"
  const description = searching
    ? "试试更换凭证名称、Prefix、权限关键词，或切换状态筛选。"
    : filteredByStatus
      ? "可以切换到“全部”查看历史凭证，或新建一个启用中的凭证。"
      : "点击右上角新建凭证，开始接入开放接口。"

  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center text-muted-foreground">
      <div className="flex size-14 items-center justify-center rounded-full bg-muted">
        {searching || filteredByStatus ? (
          <InboxIcon className="size-7" />
        ) : (
          <TerminalSquareIcon className="size-7" />
        )}
      </div>
      <div className="mt-4 text-sm font-medium text-foreground">{title}</div>
      <p className="mt-1 max-w-sm text-xs leading-5">{description}</p>
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
  disabled,
  id,
  label,
  onChange,
  placeholder,
  required = false,
  value,
}: {
  disabled?: boolean
  id: string
  label: string
  onChange: (value: string) => void
  placeholder?: string
  required?: boolean
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
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  )
}

function scopeLabel(scope: string) {
  switch (scope) {
    case "template:read":
      return "读取模板"
    case "preview:create":
      return "创建预览"
    case "print:create":
      return "创建打印"
    case "printer:read":
      return "读取打印机"
    case "job:read":
      return "任务状态"
    case "credential:manage":
      return "凭据管理"
    default:
      return scope
  }
}

function statusLabel(status: string) {
  if (status === "active") return "启用中"
  if (status === "revoked") return "已撤销"
  return status
}

function formatDateOnly(timestamp?: number | null) {
  if (!timestamp || !Number.isFinite(timestamp)) return "-"
  return new Date(timestamp * 1000).toLocaleDateString()
}

function formatDateTime(timestamp?: number | null) {
  if (!timestamp || !Number.isFinite(timestamp)) return "-"
  return new Date(timestamp * 1000).toLocaleString()
}

async function copyToken(
  token: string,
  setCopied: (copied: boolean) => void
) {
  await navigator.clipboard.writeText(token)
  setCopied(true)
  window.setTimeout(() => setCopied(false), 1500)
}
