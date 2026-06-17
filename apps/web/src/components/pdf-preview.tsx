import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  DownloadIcon,
  ExternalLinkIcon,
  Maximize2Icon,
  RotateCwIcon,
  ZoomInIcon,
  ZoomOutIcon,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type RenderedPdfPage = {
  error: string | null;
  height: number;
  pageNumber: number;
  rendering: boolean;
  url: string | null;
  width: number;
};

type PdfPreviewProps = {
  className?: string;
  emptyMessage?: ReactNode;
  loading?: boolean;
  source: string | null;
};

type ZoomMode = "fit-width" | "fit-page" | "custom";

const MIN_ZOOM_SCALE = 0.25;
const MAX_ZOOM_SCALE = 3;
const ZOOM_STEP = 0.25;
const RENDER_PIXEL_SCALE = 2.5;
const PAGE_GAP_PX = 18;
const PAGE_PADDING_PX = 24;
const PAGE_SCROLL_OFFSET_PX = 18;

let pdfjsPromise: Promise<typeof import("pdfjs-dist")> | null = null;

export function PdfPreview({
  className,
  emptyMessage = "暂无 PDF 预览。",
  loading = false,
  source,
}: PdfPreviewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const pageElementsRef = useRef<Record<number, HTMLDivElement | null>>({});
  const loadingTaskRef = useRef<import("pdfjs-dist").PDFDocumentLoadingTask | null>(null);
  const pdfDocumentRef = useRef<import("pdfjs-dist").PDFDocumentProxy | null>(null);
  const renderedPagesRef = useRef<RenderedPdfPage[]>([]);
  const pendingFocusPageRef = useRef<number | null>(null);
  const renderGenerationRef = useRef(0);
  const renderTasksRef = useRef<Record<number, { cancel: () => void } | null>>({});
  const destroyTaskRef = useRef<Promise<void>>(Promise.resolve());
  const [containerHeight, setContainerHeight] = useState(0);
  const [containerWidth, setContainerWidth] = useState(0);
  const [activePage, setActivePage] = useState(1);
  const [pageInput, setPageInput] = useState("1");
  const [pageCount, setPageCount] = useState(0);
  const [renderedPages, setRenderedPages] = useState<RenderedPdfPage[]>([]);
  const [rendering, setRendering] = useState(false);
  const [renderError, setRenderError] = useState<string | null>(null);
  const [rotation, setRotation] = useState(0);
  const [zoomMode, setZoomMode] = useState<ZoomMode>("fit-width");
  const [zoomScale, setZoomScale] = useState(1);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return undefined;

    const observer = new ResizeObserver(([entry]) => {
      const nextWidth = Math.round(entry.contentRect.width);
      const nextHeight = Math.round(entry.contentRect.height);
      setContainerWidth((current) => {
        if (Math.abs(current - nextWidth) < 12) return current;
        return nextWidth;
      });
      setContainerHeight((current) => {
        if (Math.abs(current - nextHeight) < 12) return current;
        return nextHeight;
      });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(
    () => () => {
      cancelRenderedPageTasks(renderTasksRef.current);
      revokeRenderedPages(renderedPagesRef.current);
      renderedPagesRef.current = [];
      destroyTaskRef.current = destroyPdfTask(loadingTaskRef);
    },
    [],
  );

  useEffect(() => {
    if (!source) {
      setRendering(false);
      setRenderError(null);
      setPageCount(0);
      setActivePage(1);
      if (!loading) {
        replaceRenderedPages([], renderedPagesRef, setRenderedPages);
      }
    }
  }, [loading, source]);

  useEffect(() => {
    if (renderedPages.length === 0) return;
    setActivePage((current) => clampNumber(current, 1, renderedPages.length));
  }, [renderedPages.length]);

  useEffect(() => {
    setPageInput(String(activePage));
  }, [activePage]);

  useEffect(() => {
    const element = containerRef.current;
    if (!element || renderedPages.length === 0) return undefined;

    let frame = 0;
    const updateActivePage = () => {
      frame = 0;
      const containerTop = element.getBoundingClientRect().top;
      const targetY = containerTop + PAGE_SCROLL_OFFSET_PX;
      let nearestPage = renderedPages[0]?.pageNumber ?? 1;
      let nearestDistance = Number.POSITIVE_INFINITY;

      for (const page of renderedPages) {
        const pageElement = pageElementsRef.current[page.pageNumber];
        if (!pageElement) continue;
        const rect = pageElement.getBoundingClientRect();

        if (rect.top <= targetY && rect.bottom >= targetY) {
          nearestPage = page.pageNumber;
          nearestDistance = 0;
          break;
        }

        const distance = Math.abs(rect.top - targetY);
        if (distance < nearestDistance) {
          nearestDistance = distance;
          nearestPage = page.pageNumber;
        }
      }

      setActivePage((current) => (current === nearestPage ? current : nearestPage));
      queueRenderPages([nearestPage - 1, nearestPage, nearestPage + 1]);
    };

    const scheduleUpdate = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(updateActivePage);
    };

    element.addEventListener("scroll", scheduleUpdate, { passive: true });
    scheduleUpdate();

    return () => {
      element.removeEventListener("scroll", scheduleUpdate);
      if (frame) window.cancelAnimationFrame(frame);
    };
  }, [renderedPages]);

  useEffect(() => {
    if (!source) {
      return undefined;
    }

    let cancelled = false;
    let loadingTask: import("pdfjs-dist").PDFDocumentLoadingTask | null = null;
    const generation = renderGenerationRef.current + 1;
    renderGenerationRef.current = generation;
    cancelRenderedPageTasks(renderTasksRef.current);
    pdfDocumentRef.current = null;

    const loadPdf = async () => {
      setRendering(true);
      setRenderError(null);
      const nextPages: RenderedPdfPage[] = [];

      try {
        await destroyTaskRef.current;
        if (cancelled || generation !== renderGenerationRef.current) return;
        const pdfjs = await loadPdfJs();
        if (cancelled) return;
        loadingTask = pdfjs.getDocument({ url: source });
        loadingTaskRef.current = loadingTask;
        const pdf = await loadingTask.promise;
        if (cancelled || generation !== renderGenerationRef.current) return;
        pdfDocumentRef.current = pdf;
        setPageCount(pdf.numPages);

        for (let pageNumber = 1; pageNumber <= pdf.numPages; pageNumber += 1) {
          if (cancelled) break;
          const page = await pdf.getPage(pageNumber);
          const pageRotation = normalizeRotation(page.rotate + rotation);
          const displayViewport = page.getViewport({ rotation: pageRotation, scale: 1 });

          if (cancelled) {
            page.cleanup();
            break;
          }

          nextPages.push({
            error: null,
            height: displayViewport.height,
            pageNumber,
            rendering: false,
            url: null,
            width: displayViewport.width,
          });
          page.cleanup();
        }

        if (cancelled) {
          return;
        }

        const preferredPage = clampNumber(activePage, 1, nextPages.length || 1);
        const initialPageNumbers = Array.from(new Set([preferredPage, 1, preferredPage + 1]));

        for (const pageNumber of initialPageNumbers) {
          if (cancelled || generation !== renderGenerationRef.current) return;
          const pageIndex = nextPages.findIndex((page) => page.pageNumber === pageNumber);
          if (pageIndex < 0 || nextPages[pageIndex]?.url) continue;
          const renderedPage = await renderPageToImage(pdf, pageNumber, generation);
          if (!renderedPage) continue;
          nextPages[pageIndex] = {
            ...nextPages[pageIndex],
            error: null,
            rendering: false,
            url: renderedPage.url,
          };
        }

        if (cancelled || generation !== renderGenerationRef.current) {
          revokeRenderedPages(nextPages);
          return;
        }

        replaceRenderedPages(nextPages, renderedPagesRef, setRenderedPages);
        window.requestAnimationFrame(() => {
          queueRenderPages([preferredPage - 1, preferredPage, preferredPage + 1]);
        });
      } catch (error) {
        revokeRenderedPages(nextPages);
        if (cancelled) return;
        const message = error instanceof Error ? error.message : "PDF 预览渲染失败";
        setRenderError(message);
      } finally {
        if (!cancelled) {
          setRendering(false);
        }
      }
    };

    void loadPdf();

    return () => {
      cancelled = true;
      renderGenerationRef.current += 1;
      cancelRenderedPageTasks(renderTasksRef.current);
      pdfDocumentRef.current = null;
      if (loadingTask) {
        loadingTaskRef.current = loadingTask;
      }
      destroyTaskRef.current = destroyPdfTask(loadingTaskRef);
    };
  }, [rotation, source]);

  const busy = loading || rendering;
  const hasPages = renderedPages.length > 0;
  const totalPages = pageCount || renderedPages.length;
  const firstPage = renderedPages[0] ?? null;
  const availableWidth = Math.max(280, (containerWidth || 760) - PAGE_PADDING_PX * 2);
  const availableHeight = Math.max(280, (containerHeight || 760) - PAGE_PADDING_PX * 2);
  const fitScale = firstPage
    ? clampNumber(availableWidth / firstPage.width, MIN_ZOOM_SCALE, MAX_ZOOM_SCALE)
    : 1;
  const fitPageScale = firstPage
    ? clampNumber(
        Math.min(availableWidth / firstPage.width, availableHeight / firstPage.height),
        MIN_ZOOM_SCALE,
        MAX_ZOOM_SCALE,
      )
    : fitScale;
  const displayScale = getCssScale(zoomMode, {
    fitPageScale,
    fitScale,
    zoomScale,
  });
  const zoomPercent = Math.round(displayScale * 100);
  const emptyBusyMessage = loading ? "正在生成预览..." : "正在渲染 PDF...";

  const scrollToPage = (pageNumber: number, behavior: ScrollBehavior = "smooth") => {
    const element = containerRef.current;
    const pageElement = pageElementsRef.current[pageNumber];
    if (!element || !pageElement) return;

    const containerRect = element.getBoundingClientRect();
    const pageRect = pageElement.getBoundingClientRect();
    const nextTop = element.scrollTop + pageRect.top - containerRect.top - PAGE_SCROLL_OFFSET_PX;

    setActivePage(pageNumber);
    element.scrollTo({
      behavior,
      top: Math.max(0, nextTop),
    });
  };

  useEffect(() => {
    const pageToFocus = pendingFocusPageRef.current;
    if (!pageToFocus || renderedPages.length === 0) return;
    pendingFocusPageRef.current = null;
    window.requestAnimationFrame(() => {
      scrollToPage(clampNumber(pageToFocus, 1, renderedPages.length), "auto");
    });
  }, [displayScale, renderedPages]);

  useEffect(() => {
    if (renderedPages.length === 0) return;
    queueRenderPages([activePage - 1, activePage, activePage + 1]);
  }, [activePage, renderedPages.length]);

  const changePage = (direction: -1 | 1) => {
    const nextPage = clampNumber(activePage + direction, 1, totalPages || 1);
    scrollToPage(nextPage);
  };

  const commitPageInput = () => {
    const nextPage = Number.parseInt(pageInput, 10);
    if (!Number.isFinite(nextPage)) {
      setPageInput(String(activePage));
      return;
    }
    const targetPage = clampNumber(nextPage, 1, totalPages || 1);
    setPageInput(String(targetPage));
    scrollToPage(targetPage);
  };

  const setCustomZoom = (nextScale: number) => {
    pendingFocusPageRef.current = activePage;
    setZoomMode("custom");
    setZoomScale(clampZoomToStep(nextScale));
  };

  const rotateClockwise = () => {
    pendingFocusPageRef.current = activePage;
    setRotation((current) => normalizeRotation(current + 90));
  };

  const fitWidth = () => {
    pendingFocusPageRef.current = activePage;
    setZoomMode("fit-width");
  };

  const fitPage = () => {
    pendingFocusPageRef.current = activePage;
    setZoomMode("fit-page");
  };

  const downloadPdf = () => {
    if (!source) return;
    const link = document.createElement("a");
    link.href = source;
    link.download = "template-preview.pdf";
    link.rel = "noopener noreferrer";
    link.click();
  };

  const openPdf = () => {
    if (!source) return;
    window.open(source, "_blank", "noopener,noreferrer");
  };

  function queueRenderPages(pageNumbers: number[]) {
    const pdf = pdfDocumentRef.current;
    if (!pdf) return;
    const generation = renderGenerationRef.current;
    const uniquePageNumbers = Array.from(
      new Set(pageNumbers.filter((pageNumber) => pageNumber >= 1 && pageNumber <= pdf.numPages)),
    );

    for (const pageNumber of uniquePageNumbers) {
      const pageState = renderedPagesRef.current.find((page) => page.pageNumber === pageNumber);
      if (!pageState || pageState.url || pageState.rendering || renderTasksRef.current[pageNumber]) {
        continue;
      }
      void renderPage(pageNumber, generation);
    }
  }

  async function renderPage(pageNumber: number, generation: number) {
    const pdf = pdfDocumentRef.current;
    if (!pdf) return;
    updateRenderedPage(pageNumber, (page) => ({ ...page, error: null, rendering: true }));

    let renderTask: { cancel: () => void; promise: Promise<unknown> } | null = null;
    let nextUrl: string | null = null;

    try {
      const renderedPage = await renderPageToImage(pdf, pageNumber, generation, (task) => {
        renderTask = task;
        renderTasksRef.current[pageNumber] = task;
      });
      renderTasksRef.current[pageNumber] = renderTask;
      if (!renderedPage || generation !== renderGenerationRef.current) return;
      nextUrl = renderedPage.url;
      updateRenderedPage(pageNumber, (current) => ({
        ...current,
        error: null,
        rendering: false,
        url: renderedPage.url,
      }));
      nextUrl = null;
    } catch (error) {
      if (generation !== renderGenerationRef.current) return;
      const message = error instanceof Error ? error.message : "页面渲染失败";
      updateRenderedPage(pageNumber, (current) => ({
        ...current,
        error: message,
        rendering: false,
      }));
    } finally {
      if (nextUrl) URL.revokeObjectURL(nextUrl);
      if (!renderTask || renderTasksRef.current[pageNumber] === renderTask) {
        renderTasksRef.current[pageNumber] = null;
      }
    }
  }

  async function renderPageToImage(
    pdf: import("pdfjs-dist").PDFDocumentProxy,
    pageNumber: number,
    generation: number,
    onRenderTask?: (task: { cancel: () => void; promise: Promise<unknown> }) => void,
  ): Promise<{ url: string } | null> {
    let page: import("pdfjs-dist").PDFPageProxy | null = null;
    try {
      page = await pdf.getPage(pageNumber);
      if (generation !== renderGenerationRef.current) return null;

      const pageRotation = normalizeRotation(page.rotate + rotation);
      const renderViewport = page.getViewport({
        rotation: pageRotation,
        scale: RENDER_PIXEL_SCALE,
      });
      const canvas = document.createElement("canvas");
      const context = canvas.getContext("2d");

      if (!context) {
        throw new Error("当前浏览器不支持 Canvas PDF 渲染");
      }

      canvas.width = Math.ceil(renderViewport.width);
      canvas.height = Math.ceil(renderViewport.height);

      const renderTask = page.render({
        canvas,
        canvasContext: context,
        viewport: renderViewport,
      });
      onRenderTask?.(renderTask);
      await renderTask.promise;

      if (generation !== renderGenerationRef.current) return null;

      const blob = await canvasToBlob(canvas);
      return { url: URL.createObjectURL(blob) };
    } finally {
      page?.cleanup();
    }
  }

  function updateRenderedPage(
    pageNumber: number,
    updater: (page: RenderedPdfPage) => RenderedPdfPage,
  ) {
    setRenderedPages((currentPages) => {
      const nextPages = currentPages.map((page) => {
        if (page.pageNumber !== pageNumber) return page;
        const nextPage = updater(page);
        if (page.url && page.url !== nextPage.url) {
          URL.revokeObjectURL(page.url);
        }
        return nextPage;
      });
      renderedPagesRef.current = nextPages;
      return nextPages;
    });
  }

  return (
    <div className={cn("relative flex h-full min-h-0 flex-col overflow-hidden bg-muted/30", className)}>
      {hasPages ? (
        <div className="z-10 flex flex-wrap items-center justify-between gap-2 border-b bg-background/95 px-3 py-2 backdrop-blur">
          <div className="flex items-center gap-1">
            <Button
              aria-label="上一页"
              disabled={activePage <= 1}
              onClick={() => changePage(-1)}
              size="icon-sm"
              type="button"
              variant="ghost"
            >
              <ChevronLeftIcon />
            </Button>
            <div className="flex items-center gap-1 rounded-lg border bg-muted/30 px-1.5 py-1 text-xs text-muted-foreground">
              <input
                aria-label="跳转页码"
                className="h-5 w-9 rounded-md border bg-background px-1 text-center font-semibold text-foreground tabular-nums outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
                inputMode="numeric"
                onBlur={commitPageInput}
                onChange={(event) => {
                  setPageInput(event.target.value.replace(/\D/g, "").slice(0, 4));
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.currentTarget.blur();
                  }
                }}
                value={pageInput}
              />
              <span className="whitespace-nowrap tabular-nums">/ {totalPages || 1} 页</span>
            </div>
            <Button
              aria-label="下一页"
              disabled={activePage >= totalPages}
              onClick={() => changePage(1)}
              size="icon-sm"
              type="button"
              variant="ghost"
            >
              <ChevronRightIcon />
            </Button>
          </div>

          <div className="flex flex-wrap items-center justify-end gap-1">
            <div className="flex items-center gap-1 rounded-lg border bg-muted/30 p-1">
              <Button
                aria-label="缩小"
                disabled={displayScale <= MIN_ZOOM_SCALE + 0.01}
                onClick={() => setCustomZoom(displayScale - ZOOM_STEP)}
                size="icon-xs"
                type="button"
                variant="ghost"
              >
                <ZoomOutIcon />
              </Button>
              <span className="min-w-12 text-center text-xs font-semibold tabular-nums">
                {zoomPercent}%
              </span>
              <Button
                aria-label="放大"
                disabled={displayScale >= MAX_ZOOM_SCALE - 0.01}
                onClick={() => setCustomZoom(displayScale + ZOOM_STEP)}
                size="icon-xs"
                type="button"
                variant="ghost"
              >
                <ZoomInIcon />
              </Button>
            </div>

            <Button
              aria-pressed={zoomMode === "fit-width"}
              onClick={fitWidth}
              size="sm"
              type="button"
              variant={zoomMode === "fit-width" ? "secondary" : "ghost"}
            >
              <Maximize2Icon />
              <span className="hidden sm:inline">适应宽度</span>
              <span className="sm:hidden">适应</span>
            </Button>
            <Button
              aria-pressed={zoomMode === "fit-page"}
              className="hidden md:inline-flex"
              onClick={fitPage}
              size="sm"
              type="button"
              variant={zoomMode === "fit-page" ? "secondary" : "ghost"}
            >
              适应整页
            </Button>
            <Button
              aria-pressed={zoomMode === "custom" && Math.abs(zoomScale - 1) < 0.01}
              onClick={() => setCustomZoom(1)}
              size="sm"
              type="button"
              variant={zoomMode === "custom" && Math.abs(zoomScale - 1) < 0.01 ? "secondary" : "ghost"}
            >
              100%
            </Button>
            <Button aria-label="顺时针旋转" onClick={rotateClockwise} size="icon-sm" type="button" variant="ghost">
              <RotateCwIcon />
            </Button>
            <Button aria-label="打开 PDF" onClick={openPdf} size="icon-sm" type="button" variant="ghost">
              <ExternalLinkIcon />
            </Button>
            <Button aria-label="下载 PDF" onClick={downloadPdf} size="icon-sm" type="button" variant="ghost">
              <DownloadIcon />
            </Button>
          </div>
        </div>
      ) : null}

      <div ref={containerRef} className="min-h-0 flex-1 overflow-auto">
      {hasPages ? (
        <div className="flex min-h-full w-max min-w-full flex-col items-center px-6 py-6" style={{ gap: PAGE_GAP_PX }}>
          {renderedPages.map((page) => (
            <div
              key={page.pageNumber}
              ref={(element) => {
                pageElementsRef.current[page.pageNumber] = element;
              }}
              className="relative shrink-0 overflow-hidden bg-white shadow-sm ring-1 ring-border"
              style={{
                aspectRatio: `${page.width} / ${page.height}`,
                width: page.width * displayScale,
              }}
            >
              {page.url ? (
                <img
                  src={page.url}
                  alt={`PDF 第 ${page.pageNumber} 页`}
                  className="size-full"
                />
              ) : (
                <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-white text-xs text-muted-foreground/70">
                  {page.error ? <span>{page.error}</span> : null}
                </div>
              )}
            </div>
          ))}
        </div>
      ) : (
        <div className="flex h-full min-h-[460px] flex-col items-center justify-center bg-background p-8 text-center text-sm text-muted-foreground lg:min-h-0">
          {busy ? (
            emptyBusyMessage
          ) : renderError ? (
            renderError
          ) : (
            emptyMessage
          )}
        </div>
      )}
      </div>

      {hasPages && renderError ? (
        <div className="pointer-events-none absolute inset-x-4 bottom-4 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive shadow-sm backdrop-blur">
          {renderError}
        </div>
      ) : null}
    </div>
  );
}

function clampNumber(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function clampZoomToStep(value: number) {
  return clampNumber(Math.round(value / ZOOM_STEP) * ZOOM_STEP, MIN_ZOOM_SCALE, MAX_ZOOM_SCALE);
}

function normalizeRotation(value: number) {
  return ((value % 360) + 360) % 360;
}

function getCssScale(
  zoomMode: ZoomMode,
  scales: {
    fitPageScale: number;
    fitScale: number;
    zoomScale: number;
  },
) {
  if (zoomMode === "fit-page") return scales.fitPageScale;
  if (zoomMode === "fit-width") return scales.fitScale;
  return scales.zoomScale;
}

function replaceRenderedPages(
  nextPages: RenderedPdfPage[],
  renderedPagesRef: { current: RenderedPdfPage[] },
  setRenderedPages: (pages: RenderedPdfPage[]) => void,
) {
  revokeRenderedPages(renderedPagesRef.current);
  renderedPagesRef.current = nextPages;
  setRenderedPages(nextPages);
}

function revokeRenderedPages(pages: RenderedPdfPage[]) {
  for (const page of pages) {
    if (page.url) {
      URL.revokeObjectURL(page.url);
    }
  }
}

function cancelRenderedPageTasks(tasks: Record<number, { cancel: () => void } | null>) {
  for (const task of Object.values(tasks)) {
    task?.cancel();
  }
  for (const key of Object.keys(tasks)) {
    delete tasks[Number(key)];
  }
}

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("PDF 页面渲染失败：无法导出图像"));
        return;
      }
      resolve(blob);
    }, "image/png");
  });
}

async function loadPdfJs() {
  pdfjsPromise ??= import("pdfjs-dist/webpack.mjs");
  return pdfjsPromise;
}

async function destroyPdfTask(
  loadingTaskRef: { current: import("pdfjs-dist").PDFDocumentLoadingTask | null },
) {
  const task = loadingTaskRef.current;
  loadingTaskRef.current = null;
  if (!task) return;
  try {
    await task.destroy();
  } catch {
    // Ignore teardown errors so the next preview lifecycle can continue.
  }
}
