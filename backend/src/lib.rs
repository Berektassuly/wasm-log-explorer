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
#[wasm_bindgen]
pub fn get_buffer_pointer(size: usize) -> *mut u8 {
    ENGINE
        .write()
        .expect("engine lock")
        .get_buffer_pointer(size)
}

/// Indexes the chunk of length `chunk_len` that JS wrote into the buffer. Scans for
/// newlines and appends line-start offsets. Handles lines split across chunk boundaries.
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
}

/// Returns the number of lines indexed so far.
#[wasm_bindgen]
pub fn get_line_count() -> usize {
    ENGINE.read().expect("engine lock").line_count()
}

/// Returns lines in the range [start, end) as a JS array of strings (UTF-8 decoded).
/// Only valid for ranges that have been streamed into the buffer (engine accumulates).
#[wasm_bindgen]
pub fn get_lines(start: usize, end: usize) -> JsValue {
    let engine = ENGINE.read().expect("engine lock");
    let ranges = engine.get_line_ranges(start, end);
    let arr = js_sys::Array::new();
    for (s, e) in ranges {
        let slice = engine.buffer_slice(s, e);
        let s = String::from_utf8_lossy(slice).into_owned();
        arr.push(&JsValue::from(s));
    }
    arr.into()
}

/// Clears the engine state (buffer and index). Call between file sessions to free memory.
#[wasm_bindgen]
pub fn clear() {
    ENGINE.write().expect("engine lock").clear();
}

/// Searches for `needle` (raw bytes, e.g. from Uint8Array) in all lines.
/// Returns a JS array of line indices (u32) for lines containing the needle.
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
