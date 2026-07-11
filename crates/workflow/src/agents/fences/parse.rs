//! Strict parser for the `<!-- specify:context begin -->` /
//! `<!-- specify:context end -->` document shape — emits the byte
//! body consumed by init-time context fingerprinting.

use std::collections::BTreeMap;

const OPEN_LINE: &[u8] = b"<!-- specify:context begin\n";
const OPEN_MARKER: &[u8] = b"<!-- specify:context begin";
const OPEN_END_LINE: &[u8] = b"-->";
const CLOSE_MARKER: &[u8] = b"<!-- specify:context end -->";

/// Parsed representation of a single fenced context block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedDocument<'a> {
    source: &'a [u8],
    body_start: usize,
    close_start: usize,
    metadata: BTreeMap<String, String>,
}

impl<'a> FencedDocument<'a> {
    /// Bytes between the completed opening fence and the closing fence.
    #[must_use]
    pub fn body(&self) -> &'a [u8] {
        &self.source[self.body_start..self.close_start]
    }

    /// Opening-fence metadata parsed as deterministic key order.
    #[cfg(test)]
    #[must_use]
    const fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }
}

/// Reason a fenced document could not be parsed or planned.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FenceError {
    /// More than one opening fence was present, making replacement ambiguous.
    #[error("context-malformed-fences: multiple opening fences found")]
    MultipleOpeningFences,
    /// More than one closing fence was present, making replacement ambiguous.
    #[error("context-malformed-fences: multiple closing fences found")]
    MultipleClosingFences,
    /// The opening fence was found but its metadata terminator was missing.
    #[error("context-malformed-fences: opening context fence is missing its `-->` line")]
    MissingOpeningFenceTerminator,
    /// The opening fence terminated before any key-value metadata lines.
    #[error("context-malformed-fences: opening context fence must include metadata")]
    MissingOpeningFenceMetadata,
    /// The opening fence was found but the matching closing fence was missing.
    #[error("context-malformed-fences: closing context fence not found")]
    MissingClosingFence,
    /// A metadata key appeared more than once in the opening fence.
    #[error("context-malformed-fences: duplicate opening-fence metadata key `{0}`")]
    DuplicateMetadataKey(String),
    /// A metadata line was not strict `key: value` ASCII.
    #[error("context-malformed-fences: invalid opening-fence metadata line `{0}`")]
    InvalidMetadataLine(String),
}

/// Parse a document that may contain a Specify context fence.
///
/// Returns `Ok(None)` only when no context fence markers are present at all.
///
/// # Errors
///
/// [`FenceError`] when fence markers are present but malformed —
/// multiple or unterminated opening fences, a missing closing fence, or
/// invalid opening-fence metadata.
pub fn parse_document(bytes: &[u8]) -> Result<Option<FencedDocument<'_>>, FenceError> {
    let Some(block_start) = find_subslice(bytes, OPEN_MARKER, 0) else {
        if find_valid_closing_fences(bytes, 0)?.is_empty() {
            return Ok(None);
        }
        return Err(FenceError::MissingOpeningFenceTerminator);
    };

    if find_subslice(bytes, OPEN_MARKER, block_start + OPEN_MARKER.len()).is_some() {
        return Err(FenceError::MultipleOpeningFences);
    }
    if !bytes[block_start..].starts_with(OPEN_LINE) {
        return Err(FenceError::MissingOpeningFenceTerminator);
    }

    let metadata_start = block_start + OPEN_LINE.len();
    let (metadata, body_start) = parse_opening_metadata(bytes, metadata_start)?;
    let closing_fences = find_valid_closing_fences(bytes, body_start)?;
    match closing_fences.as_slice() {
        [] => Err(FenceError::MissingClosingFence),
        [close_start] => Ok(Some(FencedDocument {
            source: bytes,
            body_start,
            close_start: *close_start,
            metadata,
        })),
        [_, ..] => Err(FenceError::MultipleClosingFences),
    }
}

fn parse_opening_metadata(
    bytes: &[u8], mut pos: usize,
) -> Result<(BTreeMap<String, String>, usize), FenceError> {
    ensure_opening_terminator_exists(bytes, pos)?;
    let mut metadata = BTreeMap::new();
    loop {
        let (line, next_pos) =
            read_line(bytes, pos).ok_or(FenceError::MissingOpeningFenceTerminator)?;
        if line == OPEN_END_LINE {
            if metadata.is_empty() {
                return Err(FenceError::MissingOpeningFenceMetadata);
            }
            return Ok((metadata, next_pos));
        }
        let (key, value) = parse_metadata_line(line)?;
        if metadata.insert(key.clone(), value).is_some() {
            return Err(FenceError::DuplicateMetadataKey(key));
        }
        pos = next_pos;
    }
}

fn ensure_opening_terminator_exists(bytes: &[u8], mut pos: usize) -> Result<(), FenceError> {
    loop {
        let (line, next_pos) =
            read_line(bytes, pos).ok_or(FenceError::MissingOpeningFenceTerminator)?;
        if line == OPEN_END_LINE {
            return Ok(());
        }
        if next_pos == bytes.len() {
            return Err(FenceError::MissingOpeningFenceTerminator);
        }
        pos = next_pos;
    }
}

fn parse_metadata_line(line: &[u8]) -> Result<(String, String), FenceError> {
    let Some(separator) = find_subslice(line, b": ", 0) else {
        return Err(invalid_metadata_line(line));
    };
    let key = &line[..separator];
    let value = &line[separator + 2..];
    if key.is_empty() || value.is_empty() || !key.iter().all(u8::is_ascii_lowercase_or_digit_hyphen)
    {
        return Err(invalid_metadata_line(line));
    }
    let key = String::from_utf8(key.to_vec()).map_err(|_err| invalid_metadata_line(line))?;
    let value = String::from_utf8(value.to_vec()).map_err(|_err| invalid_metadata_line(line))?;
    Ok((key, value))
}

fn invalid_metadata_line(line: &[u8]) -> FenceError {
    FenceError::InvalidMetadataLine(String::from_utf8_lossy(line).into_owned())
}

fn read_line(bytes: &[u8], start: usize) -> Option<(&[u8], usize)> {
    if start >= bytes.len() {
        return None;
    }
    let line = bytes[start..].iter().position(|byte| *byte == b'\n').map_or_else(
        || (&bytes[start..], bytes.len()),
        |relative_end| {
            let end = start + relative_end;
            (&bytes[start..end], end + 1)
        },
    );
    Some(line)
}

fn find_valid_closing_fences(bytes: &[u8], start: usize) -> Result<Vec<usize>, FenceError> {
    let mut positions = Vec::new();
    let mut cursor = start;
    while let Some(pos) = find_subslice(bytes, CLOSE_MARKER, cursor) {
        if is_line_start(bytes, pos) && is_line_end(bytes, pos + CLOSE_MARKER.len()) {
            positions.push(pos);
        }
        cursor = pos.checked_add(CLOSE_MARKER.len()).ok_or(FenceError::MissingClosingFence)?;
    }
    Ok(positions)
}

fn find_subslice(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if start > haystack.len() || needle.is_empty() {
        return None;
    }
    haystack[start..].windows(needle.len()).position(|pos| pos == needle).map(|pos| start + pos)
}

const fn is_line_start(bytes: &[u8], pos: usize) -> bool {
    pos == 0 || bytes[pos - 1] == b'\n'
}

const fn is_line_end(bytes: &[u8], pos: usize) -> bool {
    pos == bytes.len() || bytes[pos] == b'\n'
}

trait MetadataKeyByte {
    fn is_ascii_lowercase_or_digit_hyphen(&self) -> bool;
}

impl MetadataKeyByte for u8 {
    fn is_ascii_lowercase_or_digit_hyphen(&self) -> bool {
        self.is_ascii_lowercase() || self.is_ascii_digit() || *self == b'-'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_matrix() {
        let input = b"# hand title\n\n<!-- specify:context begin\nfingerprint: sha256:old\n-->\n\nold body\n\n<!-- specify:context end -->\n\noperator notes\n";
        let parsed = parse_document(input).expect("parse ok").expect("fences present");
        assert_eq!(parsed.body(), b"\nold body\n\n");
        assert_eq!(parsed.metadata().get("fingerprint").map(String::as_str), Some("sha256:old"));

        assert!(
            parse_document(b"# hand-authored\n\nNo managed context here.\n")
                .expect("parse ok")
                .is_none()
        );

        let error_cases: &[(&str, FenceError)] = &[
            // Unterminated opening fence.
            (
                "<!-- specify:context begin\nfingerprint: sha256:test\nbody",
                FenceError::MissingOpeningFenceTerminator,
            ),
            // Metadata line without the `: ` separator.
            (
                "<!-- specify:context begin\nfingerprint sha256:test\n-->\nbody\n<!-- specify:context end -->",
                FenceError::InvalidMetadataLine("fingerprint sha256:test".to_string()),
            ),
            // Opening fence terminated before any metadata.
            (
                "<!-- specify:context begin\n-->\nbody\n<!-- specify:context end -->",
                FenceError::MissingOpeningFenceMetadata,
            ),
            // A repeated metadata key.
            (
                "<!-- specify:context begin\nfingerprint: sha256:a\nfingerprint: sha256:b\n-->\nbody\n<!-- specify:context end -->",
                FenceError::DuplicateMetadataKey("fingerprint".to_string()),
            ),
            // Trailing space after the close marker fails the strict line end.
            (
                "<!-- specify:context begin\nfingerprint: sha256:test\n-->\nbody\n<!-- specify:context end --> \n",
                FenceError::MissingClosingFence,
            ),
            // Two opening fences.
            (
                "<!-- specify:context begin\nfingerprint: sha256:a\n-->\na\n<!-- specify:context end -->\n<!-- specify:context begin\nfingerprint: sha256:b\n-->\nb\n<!-- specify:context end -->",
                FenceError::MultipleOpeningFences,
            ),
            // One opening fence, two well-formed closing fences (ambiguous).
            (
                "<!-- specify:context begin\nfingerprint: sha256:a\n-->\nbody\n<!-- specify:context end -->\nmore\n<!-- specify:context end -->",
                FenceError::MultipleClosingFences,
            ),
            // A well-formed open with no close at all (empty-positions arm).
            (
                "<!-- specify:context begin\nfingerprint: sha256:a\n-->\nbody only\n",
                FenceError::MissingClosingFence,
            ),
            // A lone closing fence (no opening) — terminator is what's missing.
            (
                "hand notes\n<!-- specify:context end -->\n",
                FenceError::MissingOpeningFenceTerminator,
            ),
        ];
        for (input, expected) in error_cases {
            assert_eq!(
                parse_document(input.as_bytes()).expect_err("malformed fence must fail"),
                *expected,
                "input: {input:?}"
            );
        }
    }
}
