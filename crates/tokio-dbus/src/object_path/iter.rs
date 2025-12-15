use core::mem::take;
use core::str::from_utf8_unchecked;

/// An iterator over an [`ObjectPath`].
///
/// [`ObjectPath`]: crate::ObjectPath
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct Iter<'a> {
    data: &'a [u8],
}

impl<'a> Iter<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        // NB: trim leading '/'.
        Self { data: &data[1..] }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        let data = match self.data.iter().position(|b| *b == b'/') {
            Some(n) => {
                let (head, tail) = self.data.split_at(n);
                self.data = &tail[1..];
                head
            }
            None => take(&mut self.data),
        };

        Some(unsafe { from_utf8_unchecked(data) })
    }
}

impl DoubleEndedIterator for Iter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }

        let data = match self.data.iter().rposition(|b| *b == b'/') {
            Some(n) => {
                let (head, tail) = self.data.split_at(n);
                self.data = head;
                &tail[1..]
            }
            None => take(&mut self.data),
        };

        Some(unsafe { from_utf8_unchecked(data) })
    }
}
