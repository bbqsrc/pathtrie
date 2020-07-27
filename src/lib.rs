use std::{
    cmp::Ordering,
    collections::HashMap,
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
    io::{Seek, SeekFrom, Write},
    mem::size_of,
};

mod lcp;
mod node;

pub mod fst;
pub use fst::Fst;

use lcp::{find_common_prefix, Prefix};
use node::{Node, NodeBody};

#[derive(Debug)]
struct ByteKey([u8]);

pub trait Integer: Default + Display + Debug + Copy + sealed::Sealed + TryFrom<u64> {
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error>;
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
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl Integer for u64 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl Integer for u32 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}
impl Integer for u16 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}
impl Integer for u8 {
    #[inline]
    fn write_le_bytes<W: Write>(self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl<'a, T: Integer> RawEntries<'a, T> {
    #[inline(always)]
    fn new(node: &'a Node<T>, parent: Box<[u8]>, depth: usize) -> Self {
        Self {
            node,
            cur: 0,
            child_cur: 0,
            parent,
            child: None,
            depth,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EntryType {
    Child,
    Value,
}

#[derive(Debug, Clone)]
pub struct RawEntry<'a, T: Integer> {
    pub node: &'a Node<T>,
    pub parent: Box<[u8]>,
    pub depth: usize,
    pub ty: EntryType,
}

pub struct RawEntries<'a, T: Integer> {
    node: &'a Node<T>,
    cur: usize,
    child_cur: usize,
    parent: Box<[u8]>,
    child: Option<Box<RawEntries<'a, T>>>,
    depth: usize,
}

impl<'a, T: Integer> Iterator for RawEntries<'a, T> {
    type Item = RawEntry<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.node.body {
            NodeBody::Children(children) => {
                if self.cur < children.len() {
                    let parent = [&*self.parent, &*self.node.key].concat().into_boxed_slice();
                    let result = Some(RawEntry {
                        node: &children[self.cur],
                        parent,
                        depth: self.depth,
                        ty: EntryType::Child,
                    });
                    self.cur += 1;
                    return result;
                }

                loop {
                    if self.child_cur >= children.len() {
                        return None;
                    }

                    if self.child.is_none() {
                        let parent = [&*self.parent, &*self.node.key].concat().into_boxed_slice();
                        self.child = Some(Box::new(RawEntries::new(
                            &children[self.child_cur],
                            parent,
                            self.depth + 1,
                        )));
                    }

                    if let Some(value) = self.child.as_mut().unwrap().next() {
                        return Some(value);
                    } else {
                        self.child_cur += 1;
                        self.child = None;
                    }
                }
            }
            NodeBody::Value(_) => None,
        }
    }
}

pub struct Entries<'a, T: Integer> {
    node: &'a Node<T>,
    cur: usize,
    child: Option<Box<Entries<'a, T>>>,
}

impl<'a, T: Integer> Entries<'a, T> {
    #[inline(always)]
    fn new(node: &'a Node<T>) -> Self {
        Self {
            node,
            cur: 0,
            child: None,
        }
    }
}

impl<'a, T: Integer> Iterator for Entries<'a, T> {
    type Item = (Box<[u8]>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match &self.node.body {
            NodeBody::Children(children) => loop {
                if self.cur >= children.len() {
                    return None;
                }

                if self.child.is_none() {
                    self.child = Some(Box::new(Entries::new(&children[self.cur])));
                }

                if let Some(value) = self.child.as_mut().unwrap().next() {
                    let mut out = self.node.key.to_vec();
                    out.append(&mut value.0.to_vec());
                    return Some((out.into_boxed_slice(), value.1));
                } else {
                    self.cur += 1;
                    self.child = None;
                }
            },
            NodeBody::Value(value) => {
                if self.cur > 0 {
                    return None;
                }
                self.cur += 1;
                Some((self.node.key.to_vec().into_boxed_slice(), value))
            }
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PathTrie<T: Integer> {
    root: Node<T>,
}

impl<T: Integer> Default for PathTrie<T> {
    fn default() -> Self {
        PathTrie::new()
    }
}

impl<T: Integer> PathTrie<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            root: Node {
                key: vec![].into_boxed_slice(),
                body: NodeBody::Children(vec![]),
            },
        }
    }

    #[inline]
    pub fn keys<'a>(&'a self) -> impl Iterator<Item = Box<[u8]>> + 'a {
        self.entries().map(|x| x.0)
    }

    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries().map(|x| x.1)
    }

    #[inline]
    pub fn entries(&self) -> Entries<'_, T> {
        Entries::new(&self.root)
    }

    pub fn raw_entries(&self) -> RawEntries<'_, T> {
        RawEntries::new(&self.root, Default::default(), 0)
    }

    #[inline]
    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K, value: T) {
        let key = key.as_ref();
        Self::insert_inner(&mut self.root, key, value)
    }

    #[inline]
    fn get_node<K: AsRef<[u8]>>(&self, key: K) -> Option<&Node<T>> {
        let key = key.as_ref();

        self.walk(key, &self.root)
    }

    #[inline]
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<T> {
        self.get_node(key).and_then(|node| match &node.body {
            NodeBody::Children(children) => children
                .get(0)
                .filter(|x| x.key.is_empty())
                .map(|v| v.body.assert_value()),
            NodeBody::Value(v) => Some(*v),
        })
    }

    fn insert_inner(node: &mut Node<T>, key: &[u8], value: T) {
        let (result, prefix) = Self::find_prefix(key, &node);

        match (result, prefix) {
            (None, Prefix::NoMatch(_)) => {
                node.push(
                    Node {
                        key: key.to_vec().into_boxed_slice(),
                        body: NodeBody::Value(value),
                    },
                );
            }
            (Some(_), Prefix::Incomplete(0)) => {
                node.push(Node {
                    key: key.to_vec().into_boxed_slice(),
                    body: NodeBody::Value(value),
                });
            }
            (Some(i), Prefix::Incomplete(partial)) => {
                // Keep walking.
                Self::insert_inner(node.get_mut(i), &key[partial..], value);
            }
            (Some(i), Prefix::PerfectSubset(partial)) => {
                let prefix_key = node.get_mut(i).key[..partial].to_vec().into_boxed_slice();
                let key = node.get_mut(i).key[partial..].to_vec().into_boxed_slice();

                node.get_mut(i).key = prefix_key;
                let mut value = NodeBody::Value(value);
                std::mem::swap(&mut value, &mut node.get_mut(i).body);

                node.get_mut(i).push(Node { key, body: value });
            }
            (Some(i), Prefix::Divergent(partial)) => {
                node.diverge(i, partial, key, value);
            }
            (Some(i), Prefix::Exact) => {
                node.get_mut(i).set_value(value);
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn find_prefix<'a>(key: &[u8], node: &'a Node<T>) -> (Option<usize>, Prefix) {
        let mut prefix: Prefix = Prefix::NoMatch(Ordering::Equal);
        let result = match &node.body {
            NodeBody::Children(children) => children.iter().position(|x| {
                prefix = find_common_prefix(&x.key, &key);
                match prefix {
                    Prefix::NoMatch(_) => false,
                    _ => true,
                }
            }),
            NodeBody::Value(_) => None,
        };

        (result, prefix)
    }

    fn walk<'a>(&'a self, key: &[u8], node: &'a Node<T>) -> Option<&Node<T>> {
        let (result, prefix) = Self::find_prefix(key, node);

        match (result, prefix, &node.body) {
            (Some(i), Prefix::Exact, NodeBody::Children(children)) => match &children[i].body {
                NodeBody::Children(children) => children.last().filter(|x| x.key.is_empty()),
                NodeBody::Value(_) => Some(&children[i]),
            },
            (Some(i), Prefix::Incomplete(partial), NodeBody::Children(children)) => {
                self.walk(&key[partial..], &children[i])
            }
            (_, Prefix::NoMatch(_), _) => None,
            (_, Prefix::Divergent(_), _) => None,
            (_, Prefix::PerfectSubset(_), _) => None,
            unexpected => unreachable!("{:?}", unexpected),
        }
    }
}

impl<T: Integer> PathTrie<T> {
    const HEADER_SIZE: usize = size_of::<fst::Header>();
    const NODE_SIZE: usize = size_of::<fst::Node<T>>();
    const VERSION: u8 = 0;
    const ALIGNMENT: u8 = size_of::<T>() as u8;

    pub fn write_fst<W: Write + Seek>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        let mut current_parent: Box<[u8]> = Default::default();
        let node_zero_buf = vec![0; Self::NODE_SIZE].into_boxed_slice();

        let mut wip_offsets = HashMap::new();

        // Write a blank header
        writer.write_all(&[0u8; Self::HEADER_SIZE])?;
        // Write alignment nulls
        let current_pos = writer.seek(SeekFrom::Current(0))?;
        let bytes_until_aligned = current_pos % Self::ALIGNMENT as u64;
        writer.write_all(&vec![0u8; bytes_until_aligned as usize])?;

        // Write nodes innit
        for entry in self.raw_entries() {
            let span = tracing::span!(
                tracing::Level::TRACE,
                "entry",
                parent=%String::from_utf8_lossy(&*entry.parent),
                key=%String::from_utf8_lossy(&*entry.node.key),
            );
            let _guard = span.enter();

            if current_parent != entry.parent {
                let span = tracing::span!(tracing::Level::TRACE, "new-parent",);
                let _guard = span.enter();

                // Write a blank entry
                writer.write_all(&node_zero_buf)?;
                tracing::trace!(
                    "Wrote empty entry, now at: {}",
                    writer.seek(SeekFrom::Current(0))?
                );
                current_parent = entry.parent;
            }

            let wip_key = current_parent
                .iter()
                .chain(entry.node.key.iter())
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice();
            wip_offsets.insert(wip_key, writer.seek(SeekFrom::Current(0))?);

            // Check parent in wips
            if let Some(parent_offset) = wip_offsets.remove(&current_parent) {
                let span = tracing::span!(tracing::Level::TRACE, "fill-wip-offset",);

                let _guard = span.enter();
                let current_offset = writer.seek(SeekFrom::Current(0))?;
                tracing::trace!("Current offset: {}", current_offset);

                // Go to offset and write current offset
                writer.seek(SeekFrom::Start(parent_offset))?;
                let offset =
                    T::try_from(current_offset).unwrap_or_else(|_| panic!("Offset too large"));
                offset.write_le_bytes(writer)?;
                tracing::trace!(
                    "Wrote current offset, now at: {}",
                    writer.seek(SeekFrom::Current(0))?
                );

                // Return to current position
                writer.seek(SeekFrom::Start(current_offset))?;
                tracing::trace!(
                    "Returned to 'current' offset at: {}",
                    writer.seek(SeekFrom::Current(0))?
                );
            }

            let span = tracing::span!(tracing::Level::TRACE, "children");
            let _guard = span.enter();
            let len: u8 = entry
                .node
                .key
                .len()
                .try_into()
                .expect("Keys are currently limited to 255 bytes in length");

            match &entry.node.body {
                NodeBody::Children(_) => {
                    // Write zeros temporarily
                    writer.write_all(&[0; size_of::<fst::NodeOffset>()])?;
                    tracing::trace!(
                        "Wrote empty WIP offset, now at: {}",
                        writer.seek(SeekFrom::Current(0))?
                    );
                }
                NodeBody::Value(_) => {
                    // Terminus sentinel
                    writer.write_all(&[u8::MAX; size_of::<fst::NodeOffset>()])?;
                    tracing::trace!(
                        "Wrote terminus bytes, now at: {}",
                        writer.seek(SeekFrom::Current(0))?
                    );
                }
            };

            // Write value if present
            if let NodeBody::Value(value) = entry.node.body {
                value.write_le_bytes(writer)?;
                tracing::trace!(
                    "Wrote value, now at: {}",
                    writer.seek(SeekFrom::Current(0))?
                );
            }

            // Write string with u8 size
            writer.write_all(&[len])?;
            tracing::trace!(
                "Wrote len `{}`, now at: {}",
                len,
                writer.seek(SeekFrom::Current(0))?
            );

            writer.write_all(&entry.node.key)?;
            tracing::trace!(
                "Wrote `{}`, now at: {}",
                String::from_utf8_lossy(&entry.node.key),
                writer.seek(SeekFrom::Current(0))?
            );

            // Write zeros to ensure alignment
            let current_pos = writer.seek(SeekFrom::Current(0))?;
            let bytes_until_aligned = size_of::<T>() - current_pos as usize % size_of::<T>();
            tracing::trace!("Writing {} padding bytes", bytes_until_aligned);
            writer.write_all(&vec![0u8; bytes_until_aligned as usize])?;
            tracing::trace!(
                "Wrote padding, now at: {}",
                writer.seek(SeekFrom::Current(0))?
            );

            assert!(writer.seek(SeekFrom::Current(0))? as usize % size_of::<T>() == 0);
        }
        writer.write_all(&node_zero_buf)?;
        tracing::trace!(
            "Wrote end of file padding, now at: {}",
            writer.seek(SeekFrom::Current(0))?
        );

        // Seek back and write header
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&[b'\xff', b'\xdf', Self::VERSION, Self::ALIGNMENT])?;

        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Cursor};
    use memmap::Mmap;

    #[test]
    fn insert_subset() {
        let mut trie = PathTrie::<u32>::new();

        trie.insert("abcd", 1);
        println!("{:?}", trie.root);
        trie.insert("ab", 2);
        println!("{:?}", trie.root);
        trie.insert("abcdab", 3);
        println!("{:?}", trie.root);
        assert_eq!(trie.keys().count(), 3);
    }

    #[test]
    fn insert_divergent() {
        let mut trie = PathTrie::<u32>::new();

        trie.insert("ab/ano", 1);
        println!("{:?}", trie.root);
        trie.insert("ab/bap", 2);
        println!("{:?}", trie.root);
        trie.insert("ab/car", 3);
        println!("{:?}", trie.root);
        trie.insert("abcdab", 4);

        assert_eq!(trie.keys().count(), 4);
        assert_eq!(trie.get("ab/ano"), Some(1));
        assert_eq!(trie.get("ab/bap"), Some(2));
        assert_eq!(trie.get("ab/car"), Some(3));
        assert_eq!(trie.get("abcdab"), Some(4));
    }

    #[test]
    fn insert_divergent2() {
        let mut trie = PathTrie::<u32>::new();

        let paths = &[
            "bacon/sandwich/hamburger",
            "bacon/sandwich/ham",
            "bacon",
            "bacon/sandwich/ham-replacement",
            "bacon/baguette/croissant",
            "bacon/jam",
            "bacon/wat",
            "break-everything/haha",
        ];

        for (n, path) in paths.iter().enumerate() {
            trie.insert(path, n as u32 + 1);
            println!(":::{}::: {}", n, &trie.root);
            assert_eq!(trie.keys().count(), n + 1);
        }

        println!(":::::: {}", trie.root);

        assert_eq!(trie.keys().count(), paths.len());

        assert_eq!(trie.get("bacon/sandwich/hamburger"), Some(1));
        assert_eq!(trie.get("bacon/sandwich/ham"), Some(2));
        assert_eq!(trie.get("bacon/wat"), Some(7));
        assert_eq!(trie.get("break-everything/haha"), Some(8));
    }

    #[test]
    fn fst() {
        let mut trie = PathTrie::<u32>::new();

        let paths = &[
            "a/1/a", "a/1/b", "a/1/c", "b/1/a", "b/1/b", "b/1/c", "c/1/a", "c/1/b", "c/1/c",
            "a/2/a", "a/2/b", "a/2/c", "b/2/a", "b/2/b", "b/2/c", "c/2/a", "c/2/b", "c/2/c",
        ];

        for (n, path) in paths.iter().enumerate() {
            trie.insert(path, n as u32 + 1);
            assert_eq!(trie.keys().count(), n + 1);
        }

        println!("ROOT: {:#?}", trie.root);
        trie.raw_entries().for_each(|x| {
            println!(
                "D:{} {:?} [{}<>{}] {:?}",
                x.depth,
                x.ty,
                String::from_utf8_lossy(&x.parent),
                String::from_utf8_lossy(&x.node.key),
                &x.node.body
            )
        });

        let mut buf = Cursor::new(vec![]);
        trie.write_fst(&mut buf).unwrap();
        println!("{:?}", &buf);

        std::fs::write("./test.fst", buf.into_inner()).unwrap();
        let mmap =
            unsafe { Mmap::map(&File::open("./test.fst").unwrap()).unwrap() };
        let fst = fst::Fst::<u32>::new(mmap).unwrap();

        for item in paths.iter() {
            println!("MMM {}", item);
            assert_eq!(trie.get(item), fst.get(item));
        }
    }

    #[test]
    fn set_and_get() {
        let mut trie = PathTrie::<u32>::new();

        let paths = &[
            "apple/banana/carrot",
            "apple/banana/coconut",
            "apple/beans/carrot",
            "bacon/sandwich/hamburger",
            "bacon/sandwich/ham",
            "bacon/sandwich/ham-replacement",
            "bacon/baguette/croissant",
            "bacon/jam",
            "bacon/wat",
            "break-everything/haha",
            "anvil/camel",
            "apple/apple/apple",
            "apple/apple/banana",
            "apple/applf/banana",
        ];

        for (n, path) in paths.iter().enumerate() {
            trie.insert(path, n as u32 + 1);
            assert_eq!(trie.keys().count(), n + 1);
        }

        assert_eq!(trie.get("bacon/sandwich/ham-replacement"), Some(6));
        assert_eq!(trie.get("bacon/sandwich/hamburger"), Some(4));
        assert_eq!(trie.get("bacon/sandwich/ham"), Some(5));
        assert_eq!(trie.get("great/otherpath"), None);
        assert_eq!(trie.get("b"), None);

        assert_eq!(trie.keys().count(), paths.len());
        println!("{}", &trie.root);

        let mut buf = Cursor::new(vec![]);
        trie.write_fst(&mut buf).unwrap();
        println!("{:?}", &buf);

        std::fs::write("./test2.fst", buf.into_inner()).unwrap();
        let mmap =
            unsafe { Mmap::map(&File::open("./test2.fst").unwrap()).unwrap() };
        let fst = fst::Fst::<u32>::new(mmap).unwrap();

        for item in paths.iter() {
            println!("MMM {}", item);
            assert_eq!(trie.get(item), fst.get(item));
        }
    }

    #[test]
    fn empty_fst() {
        let trie = PathTrie::<u32>::new();
        let mut buf = Cursor::new(vec![]);
        trie.write_fst(&mut buf).unwrap();
        println!("{:?}", &buf);

        std::fs::write("./test-empty.fst", buf.into_inner()).unwrap();
        let mmap = unsafe {
            Mmap::map(&File::open("./test-empty.fst").unwrap()).unwrap()
        };
        let fst = fst::Fst::<u32>::new(mmap).unwrap();
        fst.get("lol");
    }
}
