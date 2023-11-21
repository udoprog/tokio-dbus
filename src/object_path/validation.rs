/// Validate an object path.
pub(super) const fn validate(bytes: &[u8]) -> bool {
    let [b'/', bytes @ ..] = bytes else {
        return false;
    };

    // Special case: "/" is a valid path.
    if bytes.is_empty() {
        return true;
    }

    let mut bytes = bytes;
    let mut component = false;

    while let [b, rest @ ..] = bytes {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => {
                component = true;
            }
            b'/' => {
                if !component {
                    return false;
                }

                component = false;
            }
            _ => {
                return false;
            }
        }

        bytes = rest;
    }

    component
}
