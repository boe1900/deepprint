import {
  BASE_URL_STORAGE_KEY,
  DEFAULT_BASE_URL,
} from "@/features/deepprint/constants"

export function getAuthBaseUrl() {
  try {
    return window.localStorage.getItem(BASE_URL_STORAGE_KEY) || DEFAULT_BASE_URL
  } catch {
    return DEFAULT_BASE_URL
  }
}
