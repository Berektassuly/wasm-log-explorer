//! Byte-level substring search. Returns line indices (not byte offsets) for matching lines.
//!
//! Uses memchr::memmem::find_iter over the whole buffer, then maps match positions
//! to line indices via binary_search on line offsets (fast for large files).

use memchr::memmem;

/// Finds all line indices (0-based) whose line content contains `needle` as a substring.
/// Uses find_iter over the full buffer, then binary_search to map byte positions to lines.
pub fn match_lines(
    buffer: &[u8],
    offsets: &[u64],
    needle: &[u8],
) -> Vec<u64> {
    if needle.is_empty() {
        return (0..offsets.len() as u64).collect();
    }
    if offsets.is_empty() || buffer.is_empty() {
        return Vec::new();
    }

    let mut line_indices: Vec<u64> = memmem::find_iter(buffer, needle)
        .map(|byte_pos| byte_pos_to_line_index(byte_pos, offsets))
        .filter(|&li| li < offsets.len() as u64)
        .collect();
    line_indices.sort_unstable();
    line_indices.dedup();
    line_indices
}

/// Maps a byte position in the file to the line index (line start offset <= pos).
#[inline(always)]
fn byte_pos_to_line_index(byte_pos: usize, offsets: &[u64]) -> u64 {
    let pos = byte_pos as u64;
    let i = offsets.partition_point(|&s| s <= pos);
    i.saturating_sub(1) as u64
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
