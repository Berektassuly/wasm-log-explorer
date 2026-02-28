# wasm-log-explorer

High-performance log streaming engine in Rust targeting **wasm32-unknown-unknown**. Handles multi-gigabyte files in the browser via streaming indexing and a shared JS/Rust buffer.

## Layout

- **backend/** — Rust/WASM log engine (this crate). Build: `cd backend && cargo build --target wasm32-unknown-unknown --release`
- **app/** — frontend (будет переименован в `frontend` при необходимости)

## Architecture

- **Zero-copy intent**: JS writes chunk data into a pre-allocated region; Rust indexes in place.
- **Streaming indexer**: Uses `memchr` (SIMD) to find `\n` / `\r\n` and stores only `u64` line-start offsets.
- **Memory**: Call `clear()` between file sessions to free the index and buffer.
- **No-string processing**: All search/indexing is on `&[u8]`; UTF-8 decoding only when returning lines to the UI.

## Build

```bash
# From backend/ — WASM release
cd backend
cargo build --target wasm32-unknown-unknown --release

# Optional: optimize with wasm-opt (binaryen)
wasm-opt -O4 target/wasm32-unknown-unknown/release/wasm_log_explorer.wasm -o target/wasm32-unknown-unknown/release/wasm_log_explorer_opt.wasm
```

Install the wasm32 target if needed:

```bash
rustup target add wasm32-unknown-unknown
```

Generate JS bindings with wasm-bindgen:

```bash
cargo install wasm-bindgen-cli
wasm-bindgen --target web target/wasm32-unknown-unknown/release/wasm_log_explorer.wasm --out-dir pkg
```

## JS API

| Function | Description |
|----------|-------------|
| `get_buffer_pointer(size)` | Returns a pointer to the next write region (at least `size` bytes). Write chunk data here. |
| `index_chunk(chunk_len)` | Indexes the chunk of length `chunk_len` just written; updates line offsets. |
| `get_line_count()` | Returns the number of lines indexed. |
| `get_lines(start, end)` | Returns a JS array of strings for lines in `[start, end)`. |
| `search(needle)` | `needle` is a `Uint8Array`; returns a JS array of matching line indices. |
| `clear()` | Clears buffer and index; call between file sessions. |

## Example (JS)

```js
const wasm = await import('./pkg/wasm_log_explorer.js');
await wasm.default(); // init WASM

const CHUNK = 64 * 1024 * 1024; // 64 MiB
const ptr = wasm.get_buffer_pointer(CHUNK);
// Copy chunk from file into WASM memory at ptr, then:
wasm.index_chunk(actualChunkLen);

const count = wasm.get_line_count();
const lines = wasm.get_lines(0, 100);
const matches = wasm.search(new TextEncoder().encode('error'));

wasm.clear(); // when done with this file
```

## Project layout (backend)

```
backend/
  Cargo.toml
  src/
    lib.rs           # wasm-bindgen FFI exports
    core/
      mod.rs
      engine.rs      # LogEngine: buffer, offsets, streaming state
    indexer/
      mod.rs
      scanner.rs     # memchr newline scan; chunk-boundary handling
    search/
      mod.rs
      matcher.rs     # byte-level substring search → line indices
```

## License

MIT OR Apache-2.0
