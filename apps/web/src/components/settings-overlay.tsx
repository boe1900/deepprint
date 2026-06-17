"use client"

import {
  ArrowLeftIcon,
  XIcon,
} from "lucide-react"
import { useSettings, type SettingsSectionId } from "@/hooks/use-settings"
import { Button } from "@/components/ui/button"
import { settingGroups, settingsSections } from "@/features/settings/config"
import type { DeepprintController } from "@/features/deepprint/controller"
import { cn } from "@/lib/utils"

export function SettingsOverlay({
  controller,
}: {
  controller: DeepprintController
}) {
  const { activeSection, isOpen, close, open } = useSettings()

  if (!isOpen) return null

  const activeSetting = settingsSections[activeSection] ?? settingGroups[0].items[0]
  const ActiveSectionComponent = activeSetting.component

  return (
    <div className="fixed inset-0 z-50 flex animate-in bg-background duration-200 fade-in">
      <div className="flex w-64 flex-shrink-0 flex-col border-r bg-sidebar/80">
        <div
          className="flex h-14 items-center gap-2 border-b px-3"
          data-tauri-drag-region
        >
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={close}
            aria-label="返回"
          >
            <ArrowLeftIcon />
          </Button>
          <div className="min-w-0">
            <h2 className="font-heading truncate text-sm font-medium text-sidebar-foreground">
              设置
            </h2>
            <p className="truncate text-xs text-muted-foreground">资源管理</p>
          </div>
        </div>

        <div className="flex flex-1 flex-col gap-5 overflow-y-auto px-3 py-3">
          {settingGroups.map((group) => (
            <div key={group.title} className="flex flex-col gap-2">
              <h4 className="px-2 text-[11px] font-medium tracking-wider text-muted-foreground/70 uppercase">
                {group.title}
              </h4>
              <nav className="flex flex-col gap-1">
                {group.items.map((menu) => (
                  <button
                    key={menu.id}
                    type="button"
                    className={cn(
                      "flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                      activeSection === menu.id
                        ? "bg-sidebar-accent text-sidebar-accent-foreground ring-1 ring-sidebar-border"
                        : "text-muted-foreground hover:bg-sidebar-accent/70 hover:text-sidebar-foreground"
                    )}
                    onClick={() => open(menu.id)}
                  >
                    <menu.icon
                      className={cn(
                        "size-4",
                        activeSection === menu.id
                          ? "text-sidebar-foreground"
                          : "text-muted-foreground"
                      )}
                    />
                    <span className="truncate">{menu.label}</span>
                  </button>
                ))}
              </nav>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-1 flex-col overflow-hidden">
        <div
          className="flex h-14 items-center justify-between gap-4 border-b px-6"
          data-tauri-drag-region
        >
          <div className="min-w-0">
            <h3 className="font-heading truncate text-base font-medium text-foreground">
              {activeSetting.label}
            </h3>
            <p className="truncate text-xs text-muted-foreground">
              {activeSetting.description}
            </p>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            onClick={close}
            aria-label="关闭设置"
          >
            <XIcon />
          </Button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <div className="mx-auto max-w-5xl">
            <ActiveSectionComponent controller={controller} />
          </div>
        </div>
      </div>
    </div>
  )
}
