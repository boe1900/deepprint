import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DeepprintController } from "@/features/deepprint/controller";
import type { TypstFontInfo } from "@/features/deepprint/types";

import { FontsSection } from "./FontsSection";

function createController(fonts: TypstFontInfo[]) {
  return {
    actions: {
      loadTypstFonts: vi.fn(),
      onDeleteTypstFont: vi.fn(),
      onInstallTypstFont: vi.fn(),
    },
    typstFonts: {
      fonts,
      loading: false,
      error: null,
      installing: false,
      deletingName: null,
    },
  } as unknown as DeepprintController;
}

describe("FontsSection", () => {
  it("filters the unified font list by search keyword", () => {
    const controller = createController([
      {
        file_name: "NotoSansSC-Regular.otf",
        size_bytes: 1024,
        modified_at_ms: null,
      },
      {
        file_name: "JetBrainsMono-Regular.ttf",
        size_bytes: 2048,
        modified_at_ms: null,
      },
      {
        file_name: "BrandScript-Regular.otf",
        size_bytes: 4096,
        modified_at_ms: null,
      },
    ]);

    render(<FontsSection controller={controller} />);

    const searchInput = screen.getByPlaceholderText("检索字体");
    fireEvent.change(searchInput, { target: { value: "jetbrains" } });

    expect(screen.getByText("JetBrainsMono-Regular")).not.toBeNull();
    expect(screen.queryByText("NotoSansSC-Regular")).toBeNull();
    expect(screen.queryByText("BrandScript-Regular")).toBeNull();
    expect(screen.getByText("已显示 1 / 3")).not.toBeNull();
    expect(screen.getByRole("button", { name: "删除" })).not.toBeNull();
  });
});
