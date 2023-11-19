/// Trim whitespace from end of bytes.
pub(crate) fn trim_end(mut bytes: &[u8]) -> &[u8] {
    while let [prefix @ .., c] = bytes {
        if !c.is_ascii_whitespace() {
            break;
        }

        bytes = prefix;
    }

    bytes
}

/// Split once at the given byte.
pub(crate) fn split_once(bytes: &[u8], byte: u8) -> Option<(&[u8], &[u8])> {
    let n = bytes.iter().position(|&c| c == byte)?;
    let (head, tail) = bytes.split_at(n);
    Some((head, &tail[1..]))
}
