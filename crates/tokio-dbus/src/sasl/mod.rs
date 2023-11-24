//! Types related to SASL authentication which D-Bus performs.

#[cfg(test)]
mod tests;

use core::fmt;

use crate::lossy_str::LossyStr;

/// A GUID sent over SASL.
#[repr(transparent)]
pub struct Guid([u8]);

impl Guid {
    #[inline]
    pub(crate) fn new(guid: &[u8]) -> &Guid {
        unsafe {
            // SAFETY: The byte slice is repr transparent over this type.
            &*(guid as *const _ as *const Guid)
        }
    }
}

impl fmt::Debug for Guid {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Guid")
            .field(&LossyStr::new(&self.0))
            .finish()
    }
}

/// A SASL message.
pub enum SaslRequest<'a> {
    /// The AUTH message.
    Auth(Auth<'a>),
}

/// A SASL message.
pub enum SaslResponse<'a> {
    /// The OK message.
    Ok(&'a Guid),
}

/// The SASL authentication method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Auth<'a> {
    /// EXTERNAL authentication with literal payload.
    External(&'a [u8]),
}

impl<'a> Auth<'a> {
    /// Construct external authentication from u32 ascii hex.
    #[cfg(all(unix, feature = "libc"))]
    pub fn external_from_uid(buf: &'a mut [u8; 32]) -> Auth<'a> {
        let id = unsafe { libc::getuid() };
        Self::external_from_u32_ascii_hex(buf, id)
    }

    /// Construct an external authentication from a u32.
    pub fn external_from_u32_ascii_hex(buf: &'a mut [u8; 32], mut id: u32) -> Auth<'a> {
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
