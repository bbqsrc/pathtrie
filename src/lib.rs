#![cfg_attr(not(feature = "std"), no_std)]

use bare_io::Write;
use core::{
    convert::TryFrom,
    fmt::{Debug, Display},
};

mod fst;
mod lcp;
mod node;
#[cfg(feature = "alloc")]
mod trie;

pub use fst::Fst;
#[cfg(feature = "alloc")]
pub use trie::PathTrie;

#[derive(Debug)]
#[repr(transparent)]
struct ByteKey([u8]);

pub trait Integer: Default + Display + Debug + Copy + sealed::Sealed + TryFrom<u64> {
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error>;
}
mod sealed {
    pub trait Sealed {}
    impl Sealed for u128 {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
}

impl Integer for u128 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl Integer for u64 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl Integer for u32 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}
impl Integer for u16 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}
impl Integer for u8 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), bare_io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}
