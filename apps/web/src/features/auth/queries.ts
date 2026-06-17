import type { QueryKey } from "@tanstack/react-query"
import { fetchMe, listApiKeys, listUsers } from "./api"
import type {
  AuthMeResponse,
  ListApiKeysResponse,
  ListUsersResponse,
} from "./types"

export const authQueryKeys = {
  root: ["auth"] as const,
  me: (baseUrl: string) => [...authQueryKeys.root, "me", baseUrl] as const,
  users: (baseUrl: string) =>
    [...authQueryKeys.root, "users", baseUrl] as const,
  apiKeys: (baseUrl: string) =>
    [...authQueryKeys.root, "api-keys", baseUrl] as const,
}

export function createAuthMeQueryOptions(baseUrl: string): {
  queryKey: QueryKey
  queryFn: () => Promise<AuthMeResponse>
} {
  return {
    queryKey: authQueryKeys.me(baseUrl),
    queryFn: () => fetchMe(baseUrl),
  }
}

export function createAuthUsersQueryOptions(baseUrl: string): {
  queryKey: QueryKey
  queryFn: () => Promise<ListUsersResponse>
} {
  return {
    queryKey: authQueryKeys.users(baseUrl),
    queryFn: () => listUsers(baseUrl),
  }
}

export function createApiKeysQueryOptions(baseUrl: string): {
  queryKey: QueryKey
  queryFn: () => Promise<ListApiKeysResponse>
} {
  return {
    queryKey: authQueryKeys.apiKeys(baseUrl),
    queryFn: () => listApiKeys(baseUrl),
  }
}
