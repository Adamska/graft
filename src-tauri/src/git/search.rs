use gix::ObjectId;
use rayon::prelude::*;

use super::wire::Chunk;

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Row-ordered commit identities and searchable text, kept on the Rust side so the
/// webview never has to decode a hundred thousand strings to answer a query.
#[derive(Default)]
pub struct GraphIndex {
    oids: Vec<ObjectId>,
    text: Vec<u8>,
    /// Two cumulative end offsets per row: summary, then author.
    ends: Vec<u32>,
}

impl GraphIndex {
    pub fn append(&mut self, chunk: &Chunk) {
        let base = self.text.len() as u32;
        self.oids.extend_from_slice(&chunk.oids);
        self.text.extend_from_slice(&chunk.strings);
        self.ends.extend(chunk.string_ends.iter().map(|end| base + end));
    }

    pub fn len(&self) -> usize {
        self.oids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.oids.is_empty()
    }

    pub fn oid(&self, row: u32) -> Option<ObjectId> {
        self.oids.get(row as usize).copied()
    }

    pub fn row_of(&self, id: &ObjectId) -> Option<u32> {
        self.oids.par_iter().position_first(|oid| oid == id).map(|row| row as u32)
    }

    /// Case-insensitive match against summary and author, plus abbreviated hash prefixes.
    pub fn search(&self, query: &str, limit: usize) -> Vec<u32> {
        let needle = query.trim().to_ascii_lowercase().into_bytes();
        if needle.is_empty() {
            return Vec::new();
        }
        let hash_prefix = (4..=40).contains(&needle.len()) && needle.iter().all(u8::is_ascii_hexdigit);

        let mut rows: Vec<u32> = (0..self.oids.len())
            .into_par_iter()
            .filter(|&row| {
                let start = if row == 0 { 0 } else { self.ends[row * 2 - 1] as usize };
                let (summary_end, author_end) = (self.ends[row * 2] as usize, self.ends[row * 2 + 1] as usize);
                contains_ignore_case(&self.text[start..summary_end], &needle)
                    || contains_ignore_case(&self.text[summary_end..author_end], &needle)
                    || (hash_prefix && has_hex_prefix(&self.oids[row], &needle))
            })
            .map(|row| row as u32)
            .collect();
        rows.truncate(limit);
        rows
    }
}

fn contains_ignore_case(haystack: &[u8], lowercase_needle: &[u8]) -> bool {
    haystack.windows(lowercase_needle.len()).any(|window| window.eq_ignore_ascii_case(lowercase_needle))
}

fn has_hex_prefix(oid: &ObjectId, lowercase_prefix: &[u8]) -> bool {
    let bytes = oid.as_bytes();
    lowercase_prefix.iter().enumerate().all(|(position, digit)| {
        let byte = bytes[position / 2];
        let nibble = if position % 2 == 0 { byte >> 4 } else { byte & 0x0F };
        HEX_DIGITS[usize::from(nibble)] == *digit
    })
}
