//! Byte-level substring search. Returns line indices (not byte offsets) for matching lines.
//!
//! Operates on raw bytes; no UTF-8 decoding. Uses memchr for fast needle scanning.

use memchr::memchr;

/// Finds all line indices (0-based) whose line content contains `needle` as a substring.
/// `buffer` is the full file bytes, `offsets` the line-start offsets from the engine.
/// Returns a Vec of line indices (u64 for wasm-bindgen compatibility).
pub fn match_lines(
    buffer: &[u8],
    offsets: &[u64],
    needle: &[u8],
) -> Vec<u64> {
    if needle.is_empty() {
        return (0..offsets.len() as u64).collect();
    }

    let mut out = Vec::new();
    for (line_idx, win) in offsets.windows(2).enumerate() {
        let start = win[0] as usize;
        let end = win[1] as usize;
        if end <= buffer.len() && contains_subslice(&buffer[start..end], needle) {
            out.push(line_idx as u64);
        }
    }
    if offsets.len() >= 2 {
        let last_start = offsets[offsets.len() - 1] as usize;
        if last_start < buffer.len() && contains_subslice(&buffer[last_start..], needle) {
            out.push((offsets.len() - 1) as u64);
        }
    } else if offsets.len() == 1 && buffer.len() > 0 && contains_subslice(buffer, needle) {
        out.push(0);
    }
    out
}

/// Returns true if `haystack` contains `needle` as a contiguous subslice.
#[inline(always)]
fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    if needle.len() == 0 {
        return true;
    }
    let first = needle[0];
    let mut search_start = 0;
    while let Some(pos) = memchr(first, &haystack[search_start..]) {
        let start = search_start + pos;
        if start + needle.len() <= haystack.len()
            && haystack[start..start + needle.len()] == *needle
        {
            return true;
        }
        search_start = start + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_lines_basic() {
        let buf = b"hello\nworld\nfoo bar\n";
        let offsets = vec![0, 6, 12, 20];
        let r = match_lines(buf, &offsets, b"world");
        assert_eq!(r, [1]);
        let r = match_lines(buf, &offsets, b"o");
        assert_eq!(r, [0, 1, 2]);
    }
}
