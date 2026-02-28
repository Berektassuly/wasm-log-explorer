/**
 * Main orchestrator: EMPTY | PARSING | EXPLORER state machine.
 * Terminal/dark theme, monospaced only, no animations.
 */

import { useCallback, useRef } from 'react';
import { useExplorerWorker } from '@/hooks/useExplorerWorker';
import { VirtualGrid } from '@/components/VirtualGrid';

export default function App() {
  const {
    state,
    lineCount,
    progress,
    error,
    loadFile,
    getRows,
    reset,
  } = useExplorerWorker();

  const inputRef = useRef<HTMLInputElement>(null);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const file = e.dataTransfer.files[0];
      if (file) loadFile(file);
    },
    [loadFile]
  );

  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
  }, []);

  const onFileSelect = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) loadFile(file);
      e.target.value = '';
    },
    [loadFile]
  );

  const onOpenClick = useCallback(() => {
    inputRef.current?.click();
  }, []);

  if (state === 'EMPTY') {
    return (
      <div
        className="h-screen w-screen flex flex-col items-center justify-center bg-zinc-950 text-zinc-400 font-mono text-sm"
        onDrop={onDrop}
        onDragOver={onDragOver}
      >
        <input
          ref={inputRef}
          type="file"
          className="hidden"
          accept="*"
          onChange={onFileSelect}
        />
        <p className="mb-2">Drop a log file here or</p>
        <button
          type="button"
          className="px-4 py-2 border border-zinc-600 rounded text-zinc-300 hover:bg-zinc-800"
          onClick={onOpenClick}
        >
          Open file
        </button>
        {error && <p className="mt-4 text-red-400">{error}</p>}
      </div>
    );
  }

  if (state === 'PARSING') {
    const p = progress;
    return (
      <div className="h-screen w-screen flex flex-col bg-zinc-950 text-zinc-400 font-mono text-sm p-6">
        <div className="shrink-0 mb-4 text-zinc-500">Parsing...</div>
        <div className="shrink-0 space-y-1">
          {p && (
            <>
              <p>Bytes: {p.bytesProcessed.toLocaleString()}</p>
              <p>Lines: {p.linesProcessed.toLocaleString()}</p>
              <p>Throughput: {(p.bytesPerSecond / 1024 / 1024).toFixed(2)} MB/s</p>
              <p>Lines/s: {Math.round(p.linesPerSecond).toLocaleString()}</p>
            </>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen w-screen flex flex-col bg-zinc-950">
      <header className="shrink-0 flex items-center justify-between border-b border-zinc-800 px-3 py-2 font-mono text-sm text-zinc-500">
        <span>Explorer</span>
        <button
          type="button"
          className="px-3 py-1 border border-zinc-600 rounded hover:bg-zinc-800 text-zinc-400"
          onClick={reset}
        >
          Close
        </button>
      </header>
      <main className="flex-1 min-h-0">
        <VirtualGrid lineCount={lineCount} getRows={getRows} />
      </main>
    </div>
  );
}
