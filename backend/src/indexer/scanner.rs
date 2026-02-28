//! SIMD-accelerated newline scanner. Finds line boundaries in raw bytes for indexing.
//!
//! Uses `memchr` for fast \n and \r\n detection. Handles the boundary problem when
//! a line is split across two chunks.

use memchr::memchr_iter;

/// Scans `chunk` for newline characters and pushes the byte offset (in file space)
/// of each line start onto `line_starts`. Handles \n and \r\n.
///
/// # Arguments
/// * `chunk` - Raw bytes of the current chunk (no UTF-8 assumption).
/// * `base_offset` - File offset of the first byte of `chunk`.
/// * `line_starts` - Output vector; each pushed value is the start offset of a new line.
/// * `chunk_starts_new_line` - If true, the first byte of `chunk` is the start of a line
///   (previous chunk ended with a newline). Pushes `base_offset` as first line start when true.
///
/// # Returns
/// `true` if `chunk` ends with a newline (so the next chunk starts a new line).
#[inline(always)]
pub fn scan_chunk(
    chunk: &[u8],
    base_offset: u64,
    line_starts: &mut Vec<u64>,
    chunk_starts_new_line: bool,
) -> bool {
    if chunk.is_empty() {
        return true;
    }

    if chunk_starts_new_line {
        line_starts.push(base_offset);
    }

    let base = base_offset as u64;

    for pos in memchr_iter(b'\n', chunk) {
        let off = base + (pos as u64);
        // Line start after this newline is the next byte. Handles both \n and \r\n.
        line_starts.push(off + 1);
    }

    // Next chunk starts a new line only if this chunk ends with \n.
    chunk.last() == Some(&b'\n')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_simple_newlines() {
        let chunk = b"a\nb\nc\n";
        let mut starts = Vec::new();
        let ends = scan_chunk(chunk, 0, &mut starts, true);
        assert!(ends);
        assert_eq!(starts, [0, 2, 4, 6]);
    }

    #[test]
    fn scan_crlf() {
        let chunk = b"a\r\nb\r\n";
        let mut starts = Vec::new();
        let ends = scan_chunk(chunk, 0, &mut starts, true);
        assert!(ends);
        assert_eq!(starts, [0, 3, 6]);
    }

    #[test]
    fn boundary_no_leading_newline() {
        // Chunk does not end with newline; \n at index 6 (\r\n)
        let chunk = b"middle\r\nend";
        let mut starts = Vec::new();
        let ends = scan_chunk(chunk, 10, &mut starts, false);
        assert!(!ends);
        assert_eq!(starts, [18]); // line start after \n (base 10 + 7 + 1)
    }
}
