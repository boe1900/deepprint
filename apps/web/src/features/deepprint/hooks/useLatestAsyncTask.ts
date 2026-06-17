import { useCallback, useEffect, useMemo, useRef } from "react";

export interface LatestAsyncTaskTicket {
  requestId: number;
  signal: AbortSignal;
  isCurrent: () => boolean;
  finish: () => void;
}

export function useLatestAsyncTask() {
  const requestIdRef = useRef(0);
  const controllerRef = useRef<AbortController | null>(null);

  const start = useCallback((): LatestAsyncTaskTicket => {
    requestIdRef.current += 1;
    controllerRef.current?.abort();

    const controller = new AbortController();
    controllerRef.current = controller;
    const requestId = requestIdRef.current;

    return {
      requestId,
      signal: controller.signal,
      isCurrent: () => requestIdRef.current === requestId,
      finish: () => {
        if (controllerRef.current === controller) {
          controllerRef.current = null;
        }
      },
    };
  }, []);

  const cancel = useCallback(() => {
    requestIdRef.current += 1;
    controllerRef.current?.abort();
    controllerRef.current = null;
  }, []);

  const isCurrent = useCallback((requestId: number) => requestIdRef.current === requestId, []);

  useEffect(
    () => () => {
      controllerRef.current?.abort();
    },
    [],
  );

  return useMemo(
    () => ({
      start,
      cancel,
      isCurrent,
    }),
    [cancel, isCurrent, start],
  );
}
