/**
 * Custom hook to initialize the Comlink worker and manage EMPTY | PARSING | EXPLORER state.
 * Progress callbacks from the worker are throttled to ~20fps to keep the main thread responsive.
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import * as Comlink from 'comlink';
import type { EngineApi, ProgressPayload } from '@/worker/engine.worker';

export type ExplorerState = 'EMPTY' | 'PARSING' | 'EXPLORER';

const PROGRESS_THROTTLE_MS = 50; // ~20fps

export function useExplorerWorker() {
  const [state, setState] = useState<ExplorerState>('EMPTY');
  const [lineCount, setLineCount] = useState(0);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const [error, setError] = useState<string | null>(null);

  const workerRef = useRef<Worker | null>(null);
  const apiRef = useRef<Comlink.Remote<EngineApi> | null>(null);
  const lastProgressTimeRef = useRef(0);
  const pendingProgressRef = useRef<ProgressPayload | null>(null);
  const rafRef = useRef<number | null>(null);

  const flushProgress = useCallback(() => {
    if (pendingProgressRef.current !== null) {
      setProgress(pendingProgressRef.current);
      pendingProgressRef.current = null;
    }
    rafRef.current = null;
  }, []);

  const throttledOnProgress = useCallback(
    (p: ProgressPayload) => {
      const now = performance.now();
      pendingProgressRef.current = p;
      if (rafRef.current !== null) return;
      if (now - lastProgressTimeRef.current >= PROGRESS_THROTTLE_MS) {
        lastProgressTimeRef.current = now;
        setProgress(p);
        pendingProgressRef.current = null;
      } else {
        rafRef.current = requestAnimationFrame(() => {
          lastProgressTimeRef.current = performance.now();
          flushProgress();
        });
      }
    },
    [flushProgress]
  );

  useEffect(() => {
    const worker = new Worker(
      new URL('@/worker/engine.worker.ts', import.meta.url),
      { type: 'module' }
    );
    workerRef.current = worker;
    apiRef.current = Comlink.wrap<EngineApi>(worker);

    return () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      worker.terminate();
      workerRef.current = null;
      apiRef.current = null;
    };
  }, []);

  const loadFile = useCallback(
    async (file: File) => {
      if (!apiRef.current) return;
      setError(null);
      setProgress(null);
      setState('PARSING');

      try {
        await apiRef.current.ingestFile(file, Comlink.proxy(throttledOnProgress));
        const count = await apiRef.current.getLineCount();
        setLineCount(count);
        setProgress(null);
        setState('EXPLORER');
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        setState('EMPTY');
      }
    },
    [throttledOnProgress]
  );

  const getRows = useCallback(async (start: number, end: number): Promise<string[]> => {
    if (!apiRef.current) return [];
    return apiRef.current.getRows(start, end);
  }, []);

  const reset = useCallback(async () => {
    if (apiRef.current) await apiRef.current.clear();
    setState('EMPTY');
    setLineCount(0);
    setProgress(null);
    setError(null);
  }, []);

  return {
    state,
    lineCount,
    progress,
    error,
    loadFile,
    getRows,
    reset,
  };
}
