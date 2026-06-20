import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react"

import {
  defaultLocale,
  localeLabels,
  locales,
  messages,
  type Locale,
  type MessageKey,
  type TranslateParams,
} from "./messages"

const STORAGE_KEY = "deepprint.locale"

type I18nContextValue = {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: MessageKey | string, params?: TranslateParams) => string
}

const I18nContext = createContext<I18nContextValue | null>(null)

export { localeLabels, locales, type Locale, type MessageKey }

const fallbackContext: I18nContextValue = {
  locale: defaultLocale,
  setLocale: () => undefined,
  t: (key, params) => translate(defaultLocale, key, params),
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(() => getInitialLocale())

  const setLocale = useCallback((nextLocale: Locale) => {
    setLocaleState(nextLocale)
    localStorage.setItem(STORAGE_KEY, nextLocale)
  }, [])

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  const value = useMemo<I18nContextValue>(
    () => ({
      locale,
      setLocale,
      t: (key, params) => translate(locale, key, params),
    }),
    [locale, setLocale]
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n() {
  return useContext(I18nContext) ?? fallbackContext
}

export function translate(
  locale: Locale,
  key: MessageKey | string,
  params?: TranslateParams
) {
  const template =
    messages[locale]?.[key] ?? messages[defaultLocale][key] ?? key

  if (!params) return template

  return template.replace(/\{(\w+)\}/g, (match, name) => {
    const value = params[name]
    return value === null || value === undefined ? match : String(value)
  })
}

export function getCurrentLocale(): Locale {
  return getStoredLocale() ?? getBrowserLocale()
}

function getInitialLocale(): Locale {
  return getStoredLocale() ?? getBrowserLocale()
}

function getStoredLocale(): Locale | null {
  try {
    return normalizeLocale(localStorage.getItem(STORAGE_KEY))
  } catch {
    return null
  }
}

function getBrowserLocale(): Locale {
  return normalizeLocale(navigator.language) ?? defaultLocale
}

function normalizeLocale(value: string | null | undefined): Locale | null {
  if (!value) return null
  const normalized = value.toLowerCase()
  return locales.find((locale) => normalized === locale || normalized.startsWith(`${locale}-`)) ?? null
}
