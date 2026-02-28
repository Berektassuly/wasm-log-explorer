/**
 * Comlink-ready Web Worker wrapping the WASM log engine.
 * Handles file ingestion via streams, indexing, and getRows(start, end).
 * All heavy work and raw data stay in this worker; only slices are sent to the main thread.
 */

import * as Comlink from 'comlink';

const CHUNK_SIZE = 2 * 1024 * 1024; // 2MB per chunk

type WasmExports = {
  get_buffer_pointer: (size: number) => number;
  index_chunk: (chunkLen: number) => void;
  get_line_count: () => number;
  get_lines: (start: number, end: number) => string[];
  clear: () => void;
  memory: WebAssembly.Memory;
};

let wasm: WasmExports | null = null;

async function loadWasm(): Promise<WasmExports> {
  if (wasm) return wasm;
  const base = (typeof import.meta !== 'undefined' && import.meta.env?.BASE_URL) || '/';
  const pkgBase = `${base.replace(/\/$/, '')}/pkg`;
  const init = (await import(/* @vite-preview allow */ `${pkgBase}/wasm_log_explorer.js`)).default;
  wasm = (await init(`${pkgBase}/wasm_log_explorer_bg.wasm`)) as WasmExports;
  return wasm;
}

export type ProgressPayload = {
  bytesProcessed: number;
  linesProcessed: number;
  bytesPerSecond: number;
  linesPerSecond: number;
};

export type EngineApi = {
  /**
   * Ingest a file: stream chunks into WASM, index, and report progress via onProgress.
   * Uses Transferable (ArrayBuffer) where possible to avoid clone overhead.
   */
  ingestFile: (file: File, onProgress: (p: ProgressPayload) => void) => Promise<void>;
  /**
   * Return only the visible slice [start, end) as string[].
   */
  getRows: (start: number, end: number) => Promise<string[]>;
  getLineCount: () => Promise<number>;
  clear: () => Promise<void>;
};

function wrapWasmCall<T>(fn: () => T): T {
  try {
    return fn();
  } catch (e) {
    throw e instanceof Error ? e : new Error(String(e));
  }
}

async function wrapAsync<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (e) {
    throw e instanceof Error ? e : new Error(String(e));
  }
}

const engineApi: EngineApi = {
  async ingestFile(file: File, onProgress: (p: ProgressPayload) => void): Promise<void> {
    return wrapAsync(async () => {
      const w = await loadWasm();
      w.clear();

      const totalSize = file.size;
      let bytesProcessed = 0;
      let linesProcessed = 0;
      let lastProgressTime = performance.now();
      let lastBytes = 0;
      let lastLines = 0;

      const stream = file.stream();
      const reader = stream.getReader();
      let buffer = new Uint8Array(CHUNK_SIZE);
      let bufferOffset = 0;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        const chunk = value as Uint8Array;
        let chunkOffset = 0;

        while (chunkOffset < chunk.length) {
          const toCopy = Math.min(chunk.length - chunkOffset, buffer.length - bufferOffset);
          buffer.set(chunk.subarray(chunkOffset, chunkOffset + toCopy), bufferOffset);
          bufferOffset += toCopy;
          chunkOffset += toCopy;

          if (bufferOffset === buffer.length) {
            const ptr = wrapWasmCall(() => w.get_buffer_pointer(bufferOffset));
            const mem = new Uint8Array(w.memory.buffer, ptr, bufferOffset);
            mem.set(buffer.subarray(0, bufferOffset));
            wrapWasmCall(() => w.index_chunk(bufferOffset));
            bytesProcessed += bufferOffset;
            linesProcessed = wrapWasmCall(() => w.get_line_count());
            bufferOffset = 0;

            const now = performance.now();
            const dt = (now - lastProgressTime) / 1000;
            if (dt >= 0.05) {
              lastProgressTime = now;
              const dBytes = bytesProcessed - lastBytes;
              const dLines = linesProcessed - lastLines;
              lastBytes = bytesProcessed;
              lastLines = linesProcessed;
              onProgress({
                bytesProcessed,
                linesProcessed,
                bytesPerSecond: dt > 0 ? dBytes / dt : 0,
                linesPerSecond: dt > 0 ? dLines / dt : 0,
              });
            }
          }
        }
      }

      if (bufferOffset > 0) {
        const ptr = wrapWasmCall(() => w.get_buffer_pointer(bufferOffset));
        const mem = new Uint8Array(w.memory.buffer, ptr, bufferOffset);
        mem.set(buffer.subarray(0, bufferOffset));
        wrapWasmCall(() => w.index_chunk(bufferOffset));
        bytesProcessed += bufferOffset;
        linesProcessed = wrapWasmCall(() => w.get_line_count());
      }

      onProgress({
        bytesProcessed: totalSize,
        linesProcessed,
        bytesPerSecond: 0,
        linesPerSecond: 0,
      });
    });
  },

  async getRows(start: number, end: number): Promise<string[]> {
    return wrapAsync(async () => {
      const w = await loadWasm();
      const count = wrapWasmCall(() => w.get_line_count());
      if (start >= count) return [];
      const endClamped = Math.min(end, count);
      if (start >= endClamped) return [];
      const arr = wrapWasmCall(() => w.get_lines(start, endClamped));
      return Array.isArray(arr) ? arr : Array.from(arr as unknown as Iterable<string>);
    });
  },

  async getLineCount(): Promise<number> {
    return wrapAsync(async () => {
      const w = await loadWasm();
      return wrapWasmCall(() => w.get_line_count());
    });
  },

  async clear(): Promise<void> {
    if (wasm) wrapWasmCall(() => wasm!.clear());
  },
};

Comlink.expose(engineApi);
