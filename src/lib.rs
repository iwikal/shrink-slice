#![warn(missing_docs)]
//! # shrink_slice
//!
//! Provides an extension trait that allows you to shrink slices, like this:
//! ```rust
//! use shrink_slice::Shrink;
//! let mut slice: &[u8] = b"Hello, world!";
//! slice.shrink(1..(slice.len() - 1));
//! assert_eq!(slice, b"ello, world");
//! ```
//! In most cases, this could be accomplished by simply indexing into the slice, and taking a new
//! reference to that. But in certain context, such reborrows are not allowed by the compiler. One
//! example is when you're dealing with a mutable slice inside a closure:
//! ```rust,compile_fail
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
//! # const BUF_LEN: usize = 100;
//! # let mut buffer = [0; BUF_LEN];
//! # let mut slice: &mut [u8] = &mut buffer;
//! #
//! use shrink_slice::Shrink;
//!
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
    impl Sealed for &str {}
    impl Sealed for &mut str {}
}

/// The extension trait that allows you to shrink a slice.
pub trait Shrink: private::Sealed {
    /// The type of slice that gets shrunk.
    type Slice: ?Sized;

    /// Shrink the slice so that it refers to a subslice of its old range.
    ///
    /// If the range is outside the bounds of `[0, self.len()]`, an error is returned.
    /// For string slices, it may also error if either end of the range lands within a multi-byte
    /// character.
    #[must_use = "consider using Shrink::shrink which panics upon error"]
    fn try_shrink<R>(&mut self, range: R) -> Result<(), ShrinkError>
    where R: SliceIndex<Self::Slice, Output = Self::Slice>;

    /// Shrink the slice so that it refers to a subslice of its old range.
    ///
    /// Panics if the range is outside the bounds of `[0, self.len()]`, or for string slices, if
    /// either end of the range lands within a multi-byte character.
    #[inline]
    #[track_caller]
    fn shrink<R>(&mut self, range: R)
    where R: SliceIndex<Self::Slice, Output = Self::Slice>,
          ShrinkError: fmt::Display,
    {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn fail(e: ShrinkError) {
            panic!("{}", e);
        }

        if let Err(e) = self.try_shrink(range) {
            fail(e);
        }
    }
}

impl<T> Shrink for &[T] {
    type Slice = [T];

    fn try_shrink<R>(&mut self, range: R) -> Result<(), ShrinkError>
    where R: SliceIndex<[T], Output = [T]>
    {
        *self = self.get(range).ok_or(ShrinkError)?;
        Ok(())
    }
}

impl<T> Shrink for &mut [T] {
    type Slice = [T];

    fn try_shrink<R>(&mut self, range: R) -> Result<(), ShrinkError>
    where R: SliceIndex<[T], Output = [T]> {
        *self = std::mem::take(self).get_mut(range).ok_or(ShrinkError)?;
        Ok(())
    }
}

impl Shrink for &str {
    type Slice = str;

    fn try_shrink<R>(&mut self, range: R) -> Result<(), ShrinkError>
    where R: SliceIndex<str, Output = str> {
        *self = self.get(range).ok_or(ShrinkError)?;
        Ok(())
    }
}

impl Shrink for &mut str {
    type Slice = str;

    fn try_shrink<R>(&mut self, range: R) -> Result<(), ShrinkError>
    where R: SliceIndex<str, Output = str> {
        *self = std::mem::take(self).get_mut(range).ok_or(ShrinkError)?;
        Ok(())
    }
}

/// This error signifies that the provided range cannot index the provided slice,
/// either because it was out of bounds, or in the case of strings, because one or more bounds
/// falls within a multi-byte character.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ShrinkError;

use core::fmt;

impl fmt::Display for ShrinkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("cannot index slice by this range")
    }
}

impl std::error::Error for ShrinkError { }

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
    fn kalm_mut() {
        let mut buffer: [u8; 13] = *b"hello, world!";
        let mut slice: &mut [u8] = &mut buffer;
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
    #[should_panic]
    fn panik_mut() {
        let mut buffer: [u8; 13] = *b"hello, world!";
        let mut slice: &mut [u8] = &mut buffer;
        slice.shrink(..(slice.len() + 1));
    }

    #[test]
    fn string() {
        let mut slice = "Hello, world!";
        slice.shrink(1..(slice.len() - 1));
        assert_eq!(slice, "ello, world");
    }

    #[test]
    fn string_mut() {
        let mut buffer = "Hello, world!".to_string();
        let mut slice = buffer.as_mut();
        slice.shrink(1..(slice.len() - 1));
        assert_eq!(slice, "ello, world");
    }

    #[test]
    #[should_panic]
    fn panik_unicode() {
        "😬".shrink(1..);
    }
}
