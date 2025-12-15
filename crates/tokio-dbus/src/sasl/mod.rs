//! Types related to SASL authentication which D-Bus performs.

#[cfg(test)]
mod tests;

/// The SASL authentication method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth<'a> {
    /// EXTERNAL authentication with literal payload.
    External(&'a [u8]),
}

impl<'a> Auth<'a> {
    /// Construct external authentication from u32 ascii hex.
    #[cfg(all(unix, feature = "libc"))]
    pub(crate) fn external_from_uid(buf: &'a mut [u8; 32]) -> Auth<'a> {
        let id = unsafe { libc::getuid() };
        Self::external_from_u32_ascii_hex(buf, id)
    }

    /// Construct an external authentication from a u32.
    #[cfg(all(unix, feature = "libc"))]
    pub(crate) fn external_from_u32_ascii_hex(buf: &'a mut [u8; 32], mut id: u32) -> Auth<'a> {
        const HEX: [u8; 16] = *b"0123456789abcdef";

        let mut n = 0;

        if id == 0 {
            buf[0] = b'0';
            buf[1] = b'0';
            n = 2;
        } else {
            while id > 0 {
                let byte = (id % 10) as u8 + b'0';
                buf[n] = HEX[(byte & 0xf) as usize];
                n += 1;
                buf[n] = HEX[(byte >> 4) as usize];
                n += 1;
                id /= 10;
            }
        }

        buf[..n].reverse();
        Auth::External(&buf[..n])
    }
}
