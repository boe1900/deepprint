import { BoxIcon, TypeIcon } from "lucide-react";
import { FontsSection } from "@/features/settings/sections/FontsSection";
import { TypstPackagesSection } from "@/features/settings/sections/TypstPackagesSection";
import type { DeepprintController } from "@/features/deepprint/controller";
import type { SettingsSectionId } from "@/hooks/use-settings";

export type SettingsSectionDefinition = {
  id: SettingsSectionId;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  component: React.ComponentType<{ controller: DeepprintController }>;
};

export const settingsSections: Record<SettingsSectionId, SettingsSectionDefinition> = {
  packages: {
    id: "packages",
    label: "Typst 包管理",
    description: "管理本地 Typst 扩展包和 preview 缓存。",
    icon: BoxIcon,
    component: TypstPackagesSection,
  },
  fonts: {
    id: "fonts",
    label: "Typst 字体管理",
    description: "统一管理 Typst 运行时可用字体。",
    icon: TypeIcon,
    component: FontsSection,
  },
};

export const settingGroups: Array<{
  title: string;
  items: SettingsSectionDefinition[];
}> = [
  {
    title: "资源管理",
    items: [settingsSections.packages, settingsSections.fonts],
  },
];
