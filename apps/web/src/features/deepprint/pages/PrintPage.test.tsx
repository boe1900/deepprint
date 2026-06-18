import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { FileStrip, UploadDropzone } from "./PrintPage";

describe("PrintPage upload controls", () => {
  it("uses a native label/input relationship for the empty upload dropzone", () => {
    const inputId = "print-file-input";
    render(
      <>
        <input id={inputId} type="file" />
        <UploadDropzone
          fileInputId={inputId}
          onDropFiles={vi.fn()}
          onSelectKeyDown={vi.fn()}
        />
      </>,
    );

    const uploadControl = screen.getByRole("button", {
      name: /选择或拖拽文件到此处/,
    });

    expect(uploadControl.tagName).toBe("LABEL");
    expect(uploadControl.getAttribute("for")).toBe(inputId);
  });

  it("keeps the add-more control associated with the file input", () => {
    const inputId = "print-file-input";
    const onAddKeyDown = vi.fn();
    render(
      <FileStrip
        activeFileId={null}
        fileInputId={inputId}
        files={[]}
        onAddKeyDown={onAddKeyDown}
        onClearAll={vi.fn()}
        onRemove={vi.fn()}
        onSelect={vi.fn()}
      />,
    );

    const addControl = screen.getByRole("button", { name: "继续添加" });

    expect(addControl.tagName).toBe("LABEL");
    expect(addControl.getAttribute("for")).toBe(inputId);

    fireEvent.keyDown(addControl, { key: "Enter" });
    expect(onAddKeyDown).toHaveBeenCalledTimes(1);
  });
});
