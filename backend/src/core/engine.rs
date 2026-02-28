//! Log streaming engine: global state and zero-copy buffer management.
//!
//! Holds the shared buffer (written by JS), the line-offset index, and
//! streaming state for boundary handling across chunks.

/// Global log engine state: single buffer + index, shared between JS and Rust.
pub struct LogEngine {
    /// Pre-allocated buffer into which JS writes chunk data. Rust reads in place (zero-copy).
    buffer: Vec<u8>,
    /// Byte offsets of each line start in the logical file (cumulative across chunks).
    /// Line `i` runs from `offsets[i]` to `offsets[i+1] - 1` (or EOF for last line).
    offsets: Vec<u64>,
    /// Total number of bytes indexed so far (file position of the start of the current chunk).
    total_bytes_indexed: u64,
    /// True if the previous chunk ended with a newline (so next chunk starts a new line).
    /// Used to handle the boundary case where a line is split across two chunks.
    last_chunk_ended_with_newline: bool,
}

impl LogEngine {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offsets: Vec::new(),
            total_bytes_indexed: 0,
            last_chunk_ended_with_newline: true,
        }
    }

    /// Reserves space for the next chunk of at least `size` bytes and returns a pointer
    /// to the start of that region (at current buffer length). JS writes chunk data here.
    /// Does not change buffer length; call `append_chunk` from `index_chunk` after writing.
    /// Caller must not cache this pointer: it is invalid after any operation that may reallocate.
    #[inline(always)]
    pub fn get_buffer_pointer(&mut self, size: usize) -> *mut u8 {
        self.buffer.reserve(size);
        unsafe { self.buffer.as_mut_ptr().add(self.buffer.len()) }
    }

    /// Appends `chunk_len` bytes to the buffer (must not exceed the size passed to
    /// `get_buffer_pointer`). Returns a slice of the newly appended chunk for indexing.
    #[inline(always)]
    pub fn append_chunk(&mut self, chunk_len: usize) -> &[u8] {
        let start = self.buffer.len();
        let new_len = start + chunk_len;
        assert!(
            new_len <= self.buffer.capacity(),
            "chunk_len exceeds reserved capacity"
        );
        unsafe { self.buffer.set_len(new_len) };
        &self.buffer[start..new_len]
    }

    /// Appends new line-start offsets from the indexer. Called by the scanner for each chunk.
    #[inline(always)]
    pub fn append_offsets(&mut self, new_offsets: &[u64]) {
        self.offsets.extend_from_slice(new_offsets);
    }

    /// Advances cumulative byte count and updates boundary state after indexing a chunk.
    /// Call `discard_buffer_after_indexing()` after this to free chunk memory (keeps only offsets).
    #[inline(always)]
    pub fn advance_after_chunk(&mut self, chunk_len: usize, ended_with_newline: bool) {
        self.total_bytes_indexed += chunk_len as u64;
        self.last_chunk_ended_with_newline = ended_with_newline;
    }

    /// Discards buffer content while keeping the line-offset index. Use after each `index_chunk`
    /// to avoid accumulating the full file in WASM memory (WASM32 address space is limited).
    /// Line content must be obtained by JS reading file byte ranges and calling decode API.
    #[inline(always)]
    pub fn discard_buffer_after_indexing(&mut self) {
        self.buffer.clear();
        self.buffer.shrink_to_fit();
    }

    #[inline(always)]
    pub fn total_bytes_indexed(&self) -> u64 {
        self.total_bytes_indexed
    }

    #[inline(always)]
    pub fn last_chunk_ended_with_newline(&self) -> bool {
        self.last_chunk_ended_with_newline
    }

    /// Number of lines (number of line-start offsets).
    #[inline(always)]
    pub fn line_count(&self) -> usize {
        self.offsets.len()
    }

    /// Immutable view of line offsets for slicing and search.
    #[inline(always)]
    pub fn offsets(&self) -> &[u64] {
        &self.offsets
    }

    /// (start, end) byte ranges for lines in [start, end). get_lines uses this to slice
    /// the buffer; valid once the full file has been streamed (buffer accumulates chunks).
    pub fn get_line_ranges(&self, start: usize, end: usize) -> Vec<(u64, u64)> {
        let offsets = self.offsets();
        let end = end.min(offsets.len());
        let start = start.min(end);
        if start >= end {
            return Vec::new();
        }
        let mut ranges = Vec::with_capacity(end - start);
        for i in start..end {
            let line_start = offsets[i];
            let line_end = offsets.get(i + 1).copied().unwrap_or(self.total_bytes_indexed);
            ranges.push((line_start, line_end));
        }
        ranges
    }

    /// Clears the index and buffer, and resets streaming state. Call between file
    /// sessions to avoid memory leaks.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.offsets.clear();
        self.total_bytes_indexed = 0;
        self.last_chunk_ended_with_newline = true;
    }

    /// Returns a slice of the internal buffer for the given byte range.
    /// Valid only when the requested range has been streamed into the buffer.
    #[inline(always)]
    pub fn buffer_slice(&self, start: u64, end: u64) -> &[u8] {
        let start = start as usize;
        let end = end as usize;
        if end <= self.buffer.len() {
            &self.buffer[start..end]
        } else {
            &[]
        }
    }

    /// Logical length of the buffer (total bytes received so far).
    #[inline(always)]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for LogEngine {
    fn default() -> Self {
        Self::new()
    }
}
