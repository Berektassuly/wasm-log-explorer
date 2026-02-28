# WASM Log Explorer (App)

High-performance local-first log explorer: React + Comlink Worker + WASM.

## Setup

1. Build the WASM module (from repo root):

   ```bash
   wasm-pack build --target web --out-dir app/public/pkg
   ```

2. From `app/`:

   ```bash
   npm install
   npm run dev
   ```

## Scripts

- `npm run dev` — Vite dev server
- `npm run build` — Production build
- `npm run build:wasm` — Build WASM into `app/public/pkg` (run from repo root or ensure `app/public/pkg` exists)

## Architecture

- **EMPTY** → Drag-drop or Open file
- **PARSING** → Terminal-style progress (MB/s, lines/s); progress throttled to ~20fps
- **EXPLORER** → Virtualized grid (react-virtuoso), rows requested via `getRows(start, end)` from the worker

Raw data and indexing run in the Web Worker + WASM; the main thread only receives row slices and progress payloads.
