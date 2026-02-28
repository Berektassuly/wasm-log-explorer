//! FFI layer for the log streaming engine. Exports for JS: buffer pointer, index chunk,
//! line count, get lines, and explicit clear.

use once_cell::sync::Lazy;
use std::sync::RwLock;
use wasm_bindgen::prelude::*;

mod core;
mod indexer;
mod search;

use core::engine::LogEngine;
use indexer::scanner::scan_chunk;
use search::matcher::match_lines;

/// Global engine instance. Single-threaded WASM implies one active log session.
static ENGINE: Lazy<RwLock<LogEngine>> = Lazy::new(|| RwLock::new(LogEngine::new()));

/// Returns a pointer to the write region for the next chunk. JS should write up to
/// `size` bytes there, then call `index_chunk(chunk_len)` with the actual length.
///
/// **Important:** Do not cache this pointer in JS. Call `get_buffer_pointer(size)` immediately
/// before each chunk write; if the buffer is reallocated (e.g. by `reserve`), a previously
/// obtained pointer becomes invalid.
#[wasm_bindgen]
pub fn get_buffer_pointer(size: usize) -> *mut u8 {
    ENGINE
        .write()
        .expect("engine lock")
        .get_buffer_pointer(size)
}

/// Indexes the chunk of length `chunk_len` that JS wrote into the buffer. Scans for
/// newlines and appends line-start offsets. Handles lines split across chunk boundaries.
/// Buffer content is discarded after indexing so only offsets are kept (avoids 10GB in WASM).
#[wasm_bindgen]
pub fn index_chunk(chunk_len: usize) {
    let mut engine = ENGINE.write().expect("engine lock");
    let base = engine.total_bytes_indexed();
    let starts_new_line = engine.last_chunk_ended_with_newline();
    let (line_starts, ends_with_newline) = {
        let chunk = engine.append_chunk(chunk_len);
        let mut line_starts = Vec::new();
        let ends = scan_chunk(chunk, base, &mut line_starts, starts_new_line);
        (line_starts, ends)
    };
    engine.append_offsets(&line_starts);
    engine.advance_after_chunk(chunk_len, ends_with_newline);
    engine.discard_buffer_after_indexing();
}

/// Returns the number of lines indexed so far.
#[wasm_bindgen]
pub fn get_line_count() -> usize {
    ENGINE.read().expect("engine lock").line_count()
}

/// Returns byte ranges (file offsets) for lines [start, end). JS must read the file
/// for these ranges and call `decode_lines_from_blob` to get strings.
#[wasm_bindgen]
pub fn get_line_byte_ranges(start: usize, end: usize) -> JsValue {
    let engine = ENGINE.read().expect("engine lock");
    let ranges = engine.get_line_ranges(start, end);
    let arr = js_sys::Array::new();
    for (s, e) in ranges {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from(s as f64));
        pair.push(&JsValue::from(e as f64));
        arr.push(&pair.into());
    }
    arr.into()
}

/// Decodes lines from a contiguous blob and relative line boundaries. UTF-8 safe:
/// avoids splitting multi-byte characters at blob boundaries.
/// `line_ends` — end offset of each line within `blob` (exclusive), so line i = blob[prev_end..line_ends[i]].
#[wasm_bindgen]
pub fn decode_lines_from_blob(blob: &js_sys::Uint8Array, line_ends: &js_sys::Uint32Array) -> JsValue {
    let blob = blob.to_vec();
    let line_ends: Vec<u32> = line_ends.to_vec();
    let arr = js_sys::Array::new();
    let mut start = 0usize;
    for &end in &line_ends {
        let end = end as usize;
        let slice = if end <= blob.len() {
            &blob[start..end]
        } else {
            &blob[start..]
        };
        let s = decode_utf8_line_slice(slice);
        arr.push(&JsValue::from(s));
        start = end;
    }
    arr.into()
}

/// Decodes a single line slice to String. Trims trailing incomplete UTF-8 (e.g. when a
/// chunk cut a multi-byte character in the middle) to avoid replacement characters.
fn decode_utf8_line_slice(slice: &[u8]) -> String {
    let valid_len = match std::str::from_utf8(slice) {
        Ok(_) => slice.len(),
        Err(e) => e.valid_up_to(),
    };
    String::from_utf8_lossy(&slice[..valid_len]).into_owned()
}

/// Clears the engine state (buffer and index). Call between file sessions to free memory.
#[wasm_bindgen]
pub fn clear() {
    ENGINE.write().expect("engine lock").clear();
}

/// Searches for `needle` (raw bytes) in all lines. Returns line indices (u32).
/// Note: Buffer is cleared after each index_chunk, so this only sees in-memory content.
/// For full-file search, use a separate flow (e.g. search per chunk during ingest).
#[wasm_bindgen]
pub fn search(needle: &js_sys::Uint8Array) -> JsValue {
    let needle = needle.to_vec();
    let engine = ENGINE.read().expect("engine lock");
    let buf = engine.buffer_slice(0, engine.buffer_len() as u64);
    let offsets = engine.offsets();
    let indices = match_lines(buf, offsets, &needle);
    let arr = js_sys::Array::new();
    for i in indices {
        arr.push(&JsValue::from(i as u32));
    }
    arr.into()
}
