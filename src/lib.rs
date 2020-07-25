#![feature(const_generics)]

use std::{cmp::Ordering, fmt::Debug};

mod lcp;
mod node;
mod fst;

use lcp::{find_common_prefix, Prefix};
use node::Node;

pub trait Integer: sealed::Sealed {}
mod sealed {
    pub trait Sealed {}
    impl Sealed for u128 {}
    impl Sealed for u64 {}
    impl Sealed for u32 {}
    impl Sealed for u16 {}
    impl Sealed for u8 {}
}

impl Integer for u128 {}
impl Integer for u64 {}
impl Integer for u32 {}
impl Integer for u16 {}
impl Integer for u8 {}

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
            cur: usize::MAX,
            child: None,
        }
    }
}

impl<'a, T: Integer> Iterator for Entries<'a, T> {
    type Item = (Box<[u8]>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur == usize::MAX {
            self.cur = 0;
            if let Some(value) = self.node.value.as_ref() {
                return Some((self.node.key.to_vec().into_boxed_slice(), value));
            }
        }

        loop {
            if self.cur >= self.node.children.len() {
                return None;
            }

            if self.child.is_none() {
                self.child = Some(Box::new(Entries::new(&self.node.children[self.cur])));
            }

            if let Some(value) = self.child.as_mut().unwrap().next() {
                let mut out = self.node.key.to_vec();
                out.append(&mut value.0.to_vec());
                return Some((out.into_boxed_slice(), value.1));
            } else {
                self.cur += 1;
                self.child = None;
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
                children: vec![],
                value: None,
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

    #[inline]
    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K, value: T) {
        let key = key.as_ref();
        Self::insert_inner(&mut self.root, key, value)
    }

    #[inline]
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<&T> {
        let key = key.as_ref();

        self.walk(key, &self.root)
            .and_then(|node| node.value.as_ref())
    }

    fn insert_inner(node: &mut Node<T>, key: &[u8], value: T) {
        let (result, prefix) = Self::find_prefix(key, &node);

        match (result, prefix) {
            (Err(i), Prefix::NoMatch(_)) => {
                node.children.insert(
                    i,
                    Node {
                        key: key.to_vec().into_boxed_slice(),
                        children: vec![],
                        value: Some(value),
                    },
                );
            }
            (Ok(i), Prefix::Incomplete(partial)) => {
                // Keep walking.
                Self::insert_inner(&mut node.children[i], &key[partial..], value);
            }
            (Ok(i), Prefix::PerfectSubset(partial)) => {
                // Key is entirely confined inside an existing key
                let parent_key = key;

                // Create child node with value and new name for old node
                {
                    let key = node.children[i].key[partial..].to_vec().into_boxed_slice();
                    let value = node.children[i].value.take();
                    node.children[i].children.push(Node {
                        key,
                        children: vec![],
                        value,
                    });
                    node.children.sort_unstable();
                }

                node.children[i].key = parent_key.to_vec().into_boxed_slice();
                node.children[i].value = Some(value);
            }
            (Ok(i), Prefix::Divergent(partial)) => {
                let parent_key = &key[..partial];

                let mut new_node = Node {
                    key: parent_key.to_vec().into_boxed_slice(),
                    children: vec![],
                    value: None,
                };

                std::mem::swap(&mut new_node, &mut node.children[i]);

                // Now new_node contains the node whose key is being split
                new_node.key = new_node.key[partial..].to_vec().into_boxed_slice();

                // Add it as a child to the new split parent
                node.children[i].children.push(new_node);

                let new_child = Node {
                    key: key[partial..].to_vec().into_boxed_slice(),
                    children: vec![],
                    value: Some(value),
                };
                node.children[i].children.push(new_child);
                node.children[i].children.sort_unstable();
            }
            (Ok(i), Prefix::Exact) => {
                node.children[i].value = Some(value);
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn find_prefix<'a>(key: &[u8], node: &'a Node<T>) -> (Result<usize, usize>, Prefix) {
        let mut prefix: Prefix = Prefix::NoMatch(Ordering::Equal);
        let result = node.children.binary_search_by(|x| {
            prefix = find_common_prefix(&x.key, &key);
            match prefix {
                Prefix::NoMatch(o) => o,
                _ => Ordering::Equal,
            }
        });
        (result, prefix)
    }

    fn walk<'a>(&'a self, key: &[u8], node: &'a Node<T>) -> Option<&Node<T>> {
        let (result, prefix) = Self::find_prefix(key, node);

        match (result, prefix) {
            (Ok(i), Prefix::Exact) => Some(&node.children[i]),
            (Ok(i), Prefix::Incomplete(partial)) => self.walk(&key[partial..], &node.children[i]),
            (_, Prefix::NoMatch(_)) => None,
            (_, Prefix::Divergent(_)) => None,
            (_, Prefix::PerfectSubset(_)) => None,
            unexpected => unreachable!("{:?}", unexpected),
        }
    }
}

// impl PathTrie<usize> {
//     fn write_fst/*<W: Write>*/(&self) { //}, writer: W) {
//         self.root.children
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

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
        ];

        for (n, path) in paths.iter().enumerate() {
            trie.insert(path, n as u32);
        }
        assert_eq!(trie.get("bacon/sandwich/ham"), Some(&4));
        assert_eq!(
            trie.get("bacon/sandwich/ham-replacement"),
            Some(&5)
        );
        assert_eq!(
            trie.get("bacon/sandwich/hamburger"),
            Some(&3)
        );
        assert_eq!(trie.get("great/otherpath"), None);
        assert_eq!(trie.get("b"), None);

        assert_eq!(trie.keys().count(), paths.len());
        println!("{:?}", trie.entries().collect::<Vec<_>>());
    }
}
