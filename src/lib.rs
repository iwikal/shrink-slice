#![warn(missing_docs)]
//! # shrink_slice
//!
//! Provides an extension trait that allows you to shrink slices, like this:
//! ```rust
//! # use shrink_slice::Shrink;
//! let mut slice: &[u8] = b"Hello, world!";
//! slice.shrink(1..(slice.len() - 1));
//! assert_eq!(slice, b"ello, world");
//! ```
//! In most cases, this could be accomplished by simply indexing into the slice, and taking a new
//! reference to that. But in certain context, such reborrows are not allowed by the compiler. One
//! example is when you're dealing with a mutable slice inside a closure:
//! ```rust,compile_fail
//! # use shrink_slice::Shrink;
//! const BUF_LEN: usize = 100;
//! let mut buffer = [0; BUF_LEN];
//! let mut slice: &mut [u8] = &mut buffer;
//!
//! let mut assign_byte = |byte| {
//!     slice[0] = byte;
//!     slice = &mut slice[1..];
//!     // ^^^^^^^^^^^^^^^^^^^^
//!     // error[E0521]: borrowed data escapes outside of closure
//! };
//!
//! for i in 0..(BUF_LEN as u8) {
//!     assign_byte(i);
//! }
//! ```
//! To get around this, you could use [`std::mem::take`] to move the slice out of the variable, and
//! then reassign it.
//! ```rust
//! # use shrink_slice::Shrink;
//! # const BUF_LEN: usize = 100;
//! # let mut buffer = [0; BUF_LEN];
//! # let mut slice: &mut [u8] = &mut buffer;
//! #
//! let mut assign_byte = |byte| {
//!     slice[0] = byte;
//!     slice = &mut std::mem::take(&mut slice)[1..];
//! };
//! ```
//!
//! That's exactly what this crate does, but in an ever so slightly more convenient package:
//! ```rust
//! # use shrink_slice::Shrink;
//! # const BUF_LEN: usize = 100;
//! # let mut buffer = [0; BUF_LEN];
//! # let mut slice: &mut [u8] = &mut buffer;
//! #
//! let mut assign_byte = |byte| {
//!     slice[0] = byte;
//!     slice.shrink(1..);
//! };
//! ```

use core::slice::SliceIndex;

mod private {
    pub trait Sealed {}
    impl<T> Sealed for &[T] {}
    impl<T> Sealed for &mut [T] {}
}

/// The extension trait that allows you to shrink a slice.
pub trait Shrink: private::Sealed {
    /// The type of item contained within the slice.
    type Item;

    /// Shrink the slice so that it refers to a subslice of its old range.
    ///
    /// If the range is outside the bounds of `[0, self.len()]`, an error is returned.
    fn try_shrink<R>(&mut self, range: R) -> Result<(), BoundsError>
    where R: SliceIndex<[Self::Item], Output = [Self::Item]>;

    /// Shrink the slice so that it refers to a subslice of its old range.
    ///
    /// Panics if the range is outside the bounds of `[0, self.len()]`.
    #[inline]
    #[track_caller]
    fn shrink<R>(&mut self, range: R)
    where R: SliceIndex<[Self::Item], Output = [Self::Item]>
    {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn fail(e: BoundsError) {
            panic!("{}", e);
        }

        if let Err(e) = self.try_shrink(range) {
            fail(e);
        }
    }
}

impl<T> Shrink for &[T] {
    type Item = T;

    fn try_shrink<R>(&mut self, range: R) -> Result<(), BoundsError>
    where R: SliceIndex<[T], Output = [T]>
    {
        *self = self.get(range).ok_or(BoundsError)?;
        Ok(())
    }
}

impl<T> Shrink for &mut [T] {
    type Item = T;

    fn try_shrink<R>(&mut self, range: R) -> Result<(), BoundsError>
    where R: SliceIndex<[T], Output = [T]> {
        *self = std::mem::take(self).get_mut(range).ok_or(BoundsError)?;
        Ok(())
    }
}

/// This error signifies that the provided range was out of bounds.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct BoundsError;

use core::fmt;

impl fmt::Debug for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("slice index out of bounds")
    }
}

impl std::error::Error for BoundsError { }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kalm() {
        let mut slice: &[u8] = b"hello, world!";
        slice.shrink(1..);
        slice.shrink(..(slice.len() - 1));

        assert_eq!(slice, b"ello, world");
    }

    #[test]
    #[should_panic]
    fn panik() {
        let mut slice: &[u8] = b"hello, world!";
        slice.shrink(..(slice.len() + 1));
    }

    #[test]
    fn mut_kalm() {
        let mut buffer: [u8; 13] = *b"hello, world!";
        let mut slice: &mut [u8] = &mut buffer;
        slice.shrink(1..);
        slice.shrink(..(slice.len() - 1));

        assert_eq!(slice, b"ello, world");
    }

    #[test]
    #[should_panic]
    fn mut_panik() {
        let mut buffer: [u8; 13] = *b"hello, world!";
        let mut slice: &mut [u8] = &mut buffer;
        slice.shrink(..100);
    }
}
