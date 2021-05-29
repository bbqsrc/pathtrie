use crate::{
    lcp::{find_common_prefix, Prefix},
    Integer,
};
use core::{
    fmt::{self, Debug},
    marker::PhantomData,
    mem::size_of,
    num::NonZeroU32,
};
use std::borrow::Cow;

#[derive(Debug)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum Error {
    #[cfg_attr(feature = "std", error("Got invalid magic bytes: {0:?}"))]
    InvalidMagicBytes([u8; 2]),

    #[cfg_attr(feature = "std", error("Invalid alignment. Required: {1}, got: {0}"))]
    InvalidAlignment(u8, usize),

    #[cfg_attr(feature = "std", error("FST too small to be valid"))]
    TooSmall,
}

pub struct Fst<'data, T> {
    data: Cow<'data, [u8]>,
    marker: PhantomData<T>,
}

impl<T> Debug for Fst<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fst<{}> {{ .. }}", core::any::type_name::<T>())
    }
}

impl<T> Fst<'_, T>
where
    T: Integer + Debug,
{
    pub fn new(data: Cow<'_, [u8]>) -> Result<Fst<'_, T>, Error> {
        if data.len() < size_of::<Header>() {
            return Err(Error::TooSmall);
        }

        let header_ptr = data.as_ptr() as *const _ as *const Header;
        let header = unsafe { &*header_ptr };

        if header.magic_bytes != [b'\xff', b'\xdf'] {
            return Err(Error::InvalidMagicBytes(header.magic_bytes));
        }

        if header.alignment != size_of::<T>() as u8 {
            return Err(Error::InvalidAlignment(header.alignment, size_of::<T>()));
        }

        Ok(Fst {
            data,
            marker: PhantomData,
        })
    }

    #[inline]
    fn node_at(&self, offset: usize) -> &Node<T> {
        tracing::trace!("Node at: {}", offset);
        println!("Len: {}", self.data.len());
        let (a, data, b) = unsafe { self.data.align_to::<T>() };
        println!("{:x?} {:x?} {:x?}", a, data, b);
        assert!(a.is_empty());
        assert!(b.is_empty());
        unsafe { &*(data.as_ptr().add(offset / size_of::<T>()) as *const Node<T>) }
    }

    #[inline]
    fn node_after(&self, node: &Node<T>) -> &Node<T> {
        #[cfg(feature = "alloc")]
        tracing::trace!("Node after: {:?}", node);
        let ptr = node as *const _ as *const u8;
        let offset_ptr = unsafe { ptr.add(node.len()) };
        tracing::trace!(
            "After offset: {}",
            offset_ptr as usize - self.data.as_ptr() as usize
        );

        unsafe { &*(offset_ptr as *const Node<T>) }
    }

    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<T> {
        let start_offset = size_of::<Header>() + (size_of::<Header>() % size_of::<T>());
        let mut key = key.as_ref();
        let mut current_node = self.node_at(start_offset);

        loop {
            #[cfg(feature = "alloc")]
            tracing::trace!(
                "Current node: {:?}; len: {}",
                &current_node,
                current_node.len()
            );

            // Try to get matching value for key parts
            let common_prefix = match current_node.value() {
                Value::Key(value_key) | Value::Final(value_key, _) => {
                    #[cfg(feature = "alloc")]
                    tracing::trace!(
                        "Comparing value '{}' with our key: '{}'",
                        String::from_utf8_lossy(&value_key),
                        String::from_utf8_lossy(&key)
                    );
                    find_common_prefix(value_key, key)
                }
                Value::None => return None,
            };
            tracing::trace!("Offset :- {:?}", common_prefix);

            match common_prefix {
                Prefix::NoMatch(_) | Prefix::PerfectSubset(_) | Prefix::Divergent(_) => {
                    // Try the next node
                    tracing::trace!("Trying next node");
                    current_node = self.node_after(current_node);
                    continue;
                }
                Prefix::Incomplete(count) => {
                    key = &key[count..];
                    #[cfg(feature = "alloc")]
                    tracing::trace!("Slicing key to: '{}'", String::from_utf8_lossy(key));
                }
                Prefix::Exact => {
                    tracing::trace!("Setting key to empty");
                    key = &[]; // Should be no more key left!
                }
            }

            match (current_node.value(), current_node.next_node.get()) {
                (Value::Final(_, value), OffsetKind::Terminating) => return Some(value),
                (Value::None, _) => return None,
                (Value::Key(_), OffsetKind::Offset(success_offset)) => {
                    let candidate_node = self.node_at(success_offset as usize);
                    current_node = candidate_node;
                }
                _ => unreachable!(),
            }
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct Header {
    magic_bytes: [u8; 2], // \xff, \xdf
    version: u8,          // 0
    alignment: u8,        // ie, are our offsets 2-byte, 4-byte or 8-byte aligned
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct NodeOffset(pub(crate) Option<NonZeroU32>);

#[derive(Debug)]
pub enum OffsetKind {
    Empty,
    Offset(u32),
    Terminating,
}

impl NodeOffset {
    #[inline]
    fn get(self) -> OffsetKind {
        match self.0 {
            Some(v) => {
                if v.get() == u32::MAX {
                    OffsetKind::Terminating
                } else {
                    OffsetKind::Offset(v.get())
                }
            }
            None => OffsetKind::Empty,
        }
    }
}

#[repr(C)]
pub(crate) struct Node<T: Integer> {
    next_node: NodeOffset, // If null, there are no values in this struct; if max u32, this is a terminus and holds a value
    raw_value: T,          // There may be more bytes after this, this is simply the minimum size.
                           // value: [u8],
}

#[cfg(feature = "alloc")]
impl<T: Integer> Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value() {
            Value::None => f.debug_struct("Node::Empty").finish(),
            Value::Final(key, value) => f
                .debug_struct("Node::Terminus")
                .field("value", &value)
                .field("key", &String::from_utf8_lossy(&key))
                .finish(),
            Value::Key(key) => f
                .debug_struct("Node::Normal")
                .field("next_node", &self.next_node.get())
                .field("key", &String::from_utf8_lossy(&key))
                .finish(),
        }
    }
}

enum Value<'a, T> {
    None,
    Key(&'a [u8]),
    Final(&'a [u8], T),
}

impl<T: Integer> Node<T> {
    #[inline]
    fn value(&self) -> Value<'_, T> {
        match self.next_node.get() {
            OffsetKind::Offset(_) => {
                // Get the length (it's a u8)
                let ptr =
                    unsafe { (self as *const Node<T> as *const u8).add(size_of::<NodeOffset>()) };
                let len: u8 = unsafe { *ptr };
                Value::Key(unsafe { core::slice::from_raw_parts(ptr.add(1), len as usize) })
            }
            OffsetKind::Terminating => {
                // Get the length (it's a u8)
                let ptr = unsafe {
                    (self as *const Node<T> as *const u8)
                        .add(size_of::<NodeOffset>() + size_of::<T>())
                };
                let len: u8 = unsafe { *ptr };
                let key = unsafe { core::slice::from_raw_parts(ptr.add(1), len as usize) };
                Value::Final(key, self.raw_value)
            }
            OffsetKind::Empty => Value::None,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        match self.next_node.get() {
            OffsetKind::Offset(_) => {
                // Get the length (it's a u8)
                let ptr = unsafe { (self as *const _ as *const u8).add(size_of::<NodeOffset>()) };
                let len: u8 = unsafe { *ptr };
                let unaligned = size_of::<NodeOffset>() + size_of::<u8>() + len as usize;
                let padding = size_of::<T>() - unaligned % size_of::<T>();
                unaligned + padding
            }
            OffsetKind::Terminating => {
                let ptr = unsafe {
                    (self as *const _ as *const u8).add(size_of::<NodeOffset>() + size_of::<T>())
                };
                let len: u8 = unsafe { *ptr };
                let unaligned =
                    size_of::<NodeOffset>() + size_of::<T>() + size_of::<u8>() + len as usize;
                let padding = size_of::<T>() - unaligned % size_of::<T>();
                unaligned + padding
            }
            OffsetKind::Empty => size_of::<Self>(),
        }
    }
}
