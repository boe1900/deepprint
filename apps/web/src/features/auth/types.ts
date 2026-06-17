export type AuthUser = {
  id: string
  username: string
  email?: string | null
  display_name: string
  role: string
  status: string
  must_change_password: boolean
  created_at: number
  updated_at: number
}

export type AuthLoginResponse = {
  authenticated: boolean
  user: AuthUser
  expires_at: number
}

export type AuthMeResponse = {
  authenticated: boolean
  login_enabled: boolean
  user: AuthUser | null
  expires_at: number | null
}

export type AuthLogoutResponse = {
  logged_out: boolean
}

export type AuthChangePasswordResponse = {
  changed: boolean
  user: AuthUser
  expires_at: number
}

export type AuthUserRole = "admin" | "operator"
export type AuthUserStatus = "active" | "disabled"

export type CreateUserRequest = {
  username: string
  password: string
  email?: string | null
  display_name?: string | null
  role?: AuthUserRole
}

export type UpdateUserRequest = {
  email?: string | null
  display_name?: string | null
  role?: AuthUserRole
  status?: AuthUserStatus
}

export type ListUsersResponse = {
  users: AuthUser[]
}

export type UserResponse = {
  user: AuthUser
}

export type ResetUserPasswordResponse = {
  user: AuthUser
}

export type DeleteUserResponse = {
  deleted: boolean
  user: AuthUser
}

export type ApiKeyScope =
  | "template:read"
  | "preview:create"
  | "print:create"
  | "printer:read"
  | "job:read"

export type ApiKeyStatus = "active" | "revoked"

export type ApiKeyRecord = {
  id: string
  name: string
  key_prefix: string
  scopes: string[]
  status: ApiKeyStatus
  created_by_user_id?: string | null
  created_at: number
  updated_at: number
  last_used_at?: number | null
  revoked_at?: number | null
  expires_at?: number | null
}

export type CreateApiKeyRequest = {
  name: string
  scopes: ApiKeyScope[]
  expires_at?: number | null
}

export type ListApiKeysResponse = {
  api_keys: ApiKeyRecord[]
}

export type CreateApiKeyResponse = {
  api_key: ApiKeyRecord
  token: string
}

export type RevokeApiKeyResponse = {
  api_key: ApiKeyRecord
}
