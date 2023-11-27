use std::slice;

use crate::proto;

use super::Signature;

/// The item yielded by the [`Iter`] iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type<'a> {
    Signature(&'a Signature),
    Array(&'a Signature),
    Struct(&'a Signature),
    Dict(&'a Signature, &'a Signature),
}

/// An iterator over a signature which yields one [`Type`] at a time.
pub struct Iter<'a> {
    iter: slice::Iter<'a, u8>,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(super) fn new(s: &'a Signature) -> Iter<'a> {
        Iter {
            iter: s.as_bytes().iter(),
        }
    }

    fn next_signature(&mut self) -> Option<&'a Signature> {
        let slice = self.iter.as_slice();
        let mut depth = 0usize;
        let mut n = 0;

        loop {
            let &b = self.iter.next()?;

            let (c, term) = match b {
                b'a' => (0, false),
                b'(' | b'{' => (1, false),
                b')' | b'}' => (-1, true),
                _ => (0, true),
            };

            depth = depth.wrapping_add_signed(c);

            n += 1;

            if term && depth == 0 {
                break;
            }
        }

        Some(unsafe { Signature::new_unchecked(&slice[..n]) })
    }

    fn next_struct(&mut self) -> Option<&'a Signature> {
        let slice = self.iter.as_slice();
        let mut depth = 1usize;
        let mut n = 0;

        loop {
            let &b = self.iter.next()?;

            depth = depth.wrapping_add_signed(match b {
                b'(' | b'{' => 1,
                b')' | b'}' => -1,
                _ => 0,
            });

            if depth == 0 {
                break;
            }

            n += 1;
        }

        Some(unsafe { Signature::new_unchecked(&slice[..n]) })
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Type<'a>;

    #[inline]
    fn next(&mut self) -> Option<Type<'a>> {
        let slice = self.iter.as_slice();
        let b = self.iter.next()?;

        Some(match proto::Type::new(*b) {
            proto::Type::ARRAY => {
                let sig = self.next_signature()?;
                Type::Array(sig)
            }
            proto::Type::OPEN_PAREN => {
                let sig = self.next_struct()?;
                Type::Struct(sig)
            }
            proto::Type::OPEN_BRACE => {
                let key = self.next_signature()?;
                let value = self.next_signature()?;

                if self.iter.next().copied() != Some(b'}') {
                    return None;
                }

                Type::Dict(key, value)
            }
            _ => Type::Signature(unsafe { Signature::new_unchecked(&slice[..1]) }),
        })
    }
}
