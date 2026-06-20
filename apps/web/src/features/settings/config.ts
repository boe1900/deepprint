import { BoxIcon, TypeIcon } from "lucide-react";
import { FontsSection } from "@/features/settings/sections/FontsSection";
import { TypstPackagesSection } from "@/features/settings/sections/TypstPackagesSection";
import type { DeepprintController } from "@/features/deepprint/controller";
import type { SettingsSectionId } from "@/hooks/use-settings";
import type { MessageKey } from "@/i18n";

export type SettingsSectionDefinition = {
  id: SettingsSectionId;
  labelKey: MessageKey;
  descriptionKey: MessageKey;
  icon: React.ComponentType<{ className?: string }>;
  component: React.ComponentType<{ controller: DeepprintController }>;
};

export const settingsSections: Record<SettingsSectionId, SettingsSectionDefinition> = {
  packages: {
    id: "packages",
    labelKey: "settings.packages.label",
    descriptionKey: "settings.packages.description",
    icon: BoxIcon,
    component: TypstPackagesSection,
  },
  fonts: {
    id: "fonts",
    labelKey: "settings.fonts.label",
    descriptionKey: "settings.fonts.description",
    icon: TypeIcon,
    component: FontsSection,
  },
};

export const settingGroups: Array<{
  titleKey: MessageKey;
  items: SettingsSectionDefinition[];
}> = [
  {
    titleKey: "settings.resourceManagement",
    items: [settingsSections.packages, settingsSections.fonts],
  },
];
