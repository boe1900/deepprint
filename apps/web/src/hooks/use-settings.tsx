"use client"

import { createContext, useContext, useState, type ReactNode } from "react"

export type SettingsSectionId =
  | "packages"
  | "fonts"

type SettingsContextType = {
  isOpen: boolean
  activeSection: SettingsSectionId
  open: (section?: SettingsSectionId) => void
  close: () => void
}

const SettingsContext = createContext<SettingsContextType | null>(null)

export function useSettings() {
  const ctx = useContext(SettingsContext)
  if (!ctx) {
    throw new Error("useSettings must be used within a SettingsProvider")
  }
  return ctx
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false)
  const [activeSection, setActiveSection] =
    useState<SettingsSectionId>("packages")

  return (
    <SettingsContext.Provider
      value={{
        isOpen,
        activeSection,
        open: (section) => {
          if (section) setActiveSection(section)
          setIsOpen(true)
        },
        close: () => setIsOpen(false),
      }}
    >
      {children}
    </SettingsContext.Provider>
  )
}
