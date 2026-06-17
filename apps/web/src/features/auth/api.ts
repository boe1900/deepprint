import { requestJson } from "@/features/deepprint/api"
import type {
  AuthChangePasswordResponse,
  AuthLoginResponse,
  AuthLogoutResponse,
  AuthMeResponse,
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  CreateUserRequest,
  DeleteUserResponse,
  ListApiKeysResponse,
  ListUsersResponse,
  RevokeApiKeyResponse,
  ResetUserPasswordResponse,
  UpdateUserRequest,
  UserResponse,
} from "./types"

const AUTH_TIMEOUT_MS = 10_000

export function login(baseUrl: string, username: string, password: string) {
  return requestJson<AuthLoginResponse>(baseUrl, "/v1/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password }),
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function logout(baseUrl: string) {
  return requestJson<AuthLogoutResponse>(baseUrl, "/v1/auth/logout", {
    method: "POST",
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function changePassword(
  baseUrl: string,
  currentPassword: string,
  newPassword: string
) {
  return requestJson<AuthChangePasswordResponse>(
    baseUrl,
    "/v1/auth/change-password",
    {
      method: "POST",
      body: JSON.stringify({
        current_password: currentPassword,
        new_password: newPassword,
      }),
      timeoutMs: AUTH_TIMEOUT_MS,
    }
  )
}

export function fetchMe(baseUrl: string) {
  return requestJson<AuthMeResponse>(baseUrl, "/v1/auth/me", {
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function listUsers(baseUrl: string) {
  return requestJson<ListUsersResponse>(baseUrl, "/v1/users", {
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function createUser(baseUrl: string, request: CreateUserRequest) {
  return requestJson<UserResponse>(baseUrl, "/v1/users/create", {
    method: "POST",
    body: JSON.stringify(request),
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function updateUser(
  baseUrl: string,
  userId: string,
  request: UpdateUserRequest
) {
  return requestJson<UserResponse>(
    baseUrl,
    `/v1/users/${encodeURIComponent(userId)}/update`,
    {
      method: "POST",
      body: JSON.stringify(request),
      timeoutMs: AUTH_TIMEOUT_MS,
    }
  )
}

export function resetUserPassword(
  baseUrl: string,
  userId: string,
  password: string
) {
  return requestJson<ResetUserPasswordResponse>(
    baseUrl,
    `/v1/users/${encodeURIComponent(userId)}/reset-password`,
    {
      method: "POST",
      body: JSON.stringify({ password }),
      timeoutMs: AUTH_TIMEOUT_MS,
    }
  )
}

export function deleteUser(baseUrl: string, userId: string) {
  return requestJson<DeleteUserResponse>(
    baseUrl,
    `/v1/users/${encodeURIComponent(userId)}/delete`,
    {
      method: "POST",
      timeoutMs: AUTH_TIMEOUT_MS,
    }
  )
}

export function listApiKeys(baseUrl: string) {
  return requestJson<ListApiKeysResponse>(baseUrl, "/v1/api-keys", {
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function createApiKey(baseUrl: string, request: CreateApiKeyRequest) {
  return requestJson<CreateApiKeyResponse>(baseUrl, "/v1/api-keys/create", {
    method: "POST",
    body: JSON.stringify(request),
    timeoutMs: AUTH_TIMEOUT_MS,
  })
}

export function revokeApiKey(baseUrl: string, apiKeyId: string) {
  return requestJson<RevokeApiKeyResponse>(
    baseUrl,
    `/v1/api-keys/${encodeURIComponent(apiKeyId)}/revoke`,
    {
      method: "POST",
      timeoutMs: AUTH_TIMEOUT_MS,
    }
  )
}
