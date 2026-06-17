import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PdfPreview } from "@/components/pdf-preview";

type Deferred<T> = {
  promise: Promise<T>;
  reject: (reason?: unknown) => void;
  resolve: (value: T | PromiseLike<T>) => void;
};

function createDeferred<T>() {
  let resolve!: Deferred<T>["resolve"];
  let reject!: Deferred<T>["reject"];
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, reject, resolve };
}

describe("PdfPreview", () => {
  it("waits for the previous loading task to destroy before recreating the worker on rotate", async () => {
    const firstDestroy = createDeferred<void>();
    const getDocumentCalls: string[] = [];

    const createPage = () => ({
      cleanup: vi.fn(),
      getViewport: vi.fn(({ rotation, scale }: { rotation: number; scale: number }) => ({
        height: 200 * scale + rotation,
        width: 100 * scale + rotation,
      })),
      render: vi.fn(() => ({
        cancel: vi.fn(),
        promise: Promise.resolve(),
      })),
      rotate: 0,
    });

    const createPdf = () => ({
      destroy: vi.fn(),
      getPage: vi.fn(async () => createPage()),
      numPages: 1,
    });

    const firstTask = {
      destroy: vi.fn(() => firstDestroy.promise),
      promise: Promise.resolve(createPdf()),
    };
    const secondTask = {
      destroy: vi.fn(() => Promise.resolve()),
      promise: Promise.resolve(createPdf()),
    };

    const getDocument = vi.fn(({ url }: { url: string }) => {
      getDocumentCalls.push(url);
      return getDocumentCalls.length === 1 ? firstTask : secondTask;
    });

    vi.doMock("pdfjs-dist/webpack.mjs", () => ({
      getDocument,
    }));

    const { unmount } = render(<PdfPreview source="/preview.pdf" />);

    await waitFor(() => {
      expect(getDocument).toHaveBeenCalledTimes(1);
    });

    const rotateButton = await screen.findByRole("button", { name: "顺时针旋转" });
    fireEvent.click(rotateButton);

    await waitFor(() => {
      expect(firstTask.destroy).toHaveBeenCalledTimes(1);
    });
    expect(getDocument).toHaveBeenCalledTimes(1);

    firstDestroy.resolve();

    await waitFor(() => {
      expect(getDocument).toHaveBeenCalledTimes(2);
    });

    unmount();

    vi.doUnmock("pdfjs-dist/webpack.mjs");
  });
});
