import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

afterEach(() => {
  cleanup();
});

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

Object.defineProperty(globalThis, "ResizeObserver", {
  configurable: true,
  writable: true,
  value: ResizeObserverMock,
});

Object.defineProperty(globalThis.HTMLCanvasElement.prototype, "getContext", {
  configurable: true,
  writable: true,
  value: vi.fn(() => ({})),
});

Object.defineProperty(globalThis.HTMLCanvasElement.prototype, "toBlob", {
  configurable: true,
  writable: true,
  value(callback: (blob: Blob | null) => void) {
    callback(new Blob(["png"], { type: "image/png" }));
  },
});

if (!globalThis.URL.createObjectURL) {
  Object.defineProperty(globalThis.URL, "createObjectURL", {
    configurable: true,
    writable: true,
    value: vi.fn(() => "blob:mock-url"),
  });
}

if (!globalThis.URL.revokeObjectURL) {
  Object.defineProperty(globalThis.URL, "revokeObjectURL", {
    configurable: true,
    writable: true,
    value: vi.fn(),
  });
}

