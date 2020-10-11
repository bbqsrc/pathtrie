use crate::Integer;
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display, Write},
};
use indenter::indented;

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec, vec::Vec};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "alloc")]
pub enum NodeBody<T: Integer> {
    Children(Vec<Node<T>>),
    Value(T),
}

#[cfg(feature = "alloc")]
impl<T: Integer> NodeBody<T> {
    pub(crate) fn assert_value(&self) -> T {
        match self {
            NodeBody::Children(_) => panic!(),
            NodeBody::Value(v) => *v,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "alloc")]
pub struct Node<T: Integer> {
    pub(crate) key: Box<[u8]>,
    pub(crate) body: NodeBody<T>,
}

#[cfg(feature = "alloc")]
impl<T: Integer> Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut x = f.debug_struct("Node");

        match core::str::from_utf8(&self.key) {
            Ok(v) => x.field("key", &v),
            Err(_) => x.field("key", &self.key),
        };
        x.field("body", &self.body).finish()
    }
}

#[cfg(feature = "alloc")]
impl<T: Integer> Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match core::str::from_utf8(&self.key) {
            Ok(v) => f.write_fmt(format_args!("{:?}", v)),
            Err(_) => f.write_fmt(format_args!("{:?}", &self.key)),
        }?;

        f.write_str(" => ")?;

        match &self.body {
            NodeBody::Children(children) => {
                f.write_str("[")?;
                for child in children.iter() {
                    writeln!(f)?;
                    let mut indent =
                        indented(f).with_format(indenter::Format::Uniform { indentation: "  " });
                    write!(indent, "{}", child)?
                }
                f.write_str("\n],")?;
            }
            NodeBody::Value(value) => {
                f.write_fmt(format_args!("{},", value))?;
            }
        };

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl<T: Integer> Node<T> {
    pub(crate) fn diverge(&mut self, index: usize, partial: usize, key: &[u8], value: T) {
        let subnode = self.get_mut(index);

        let key_prefix = key[..partial].to_vec().into_boxed_slice();
        let skey = subnode.key[partial..].to_vec().into_boxed_slice();
        match &mut subnode.body {
            NodeBody::Children(children) => {
                let mut swap = Vec::new();
                core::mem::swap(&mut swap, children);
                subnode.push(Node {
                    key: skey,
                    body: NodeBody::Children(swap),
                });
            }
            NodeBody::Value(_) => {
                subnode.convert_value_to_children(skey);
            }
        }

        subnode.push(Node {
            key: key[partial..].to_vec().into_boxed_slice(),
            body: NodeBody::Value(value),
        });

        subnode.key = key_prefix;

        match &mut self.body {
            NodeBody::Children(children) => {
                children.sort_unstable();
            }
            NodeBody::Value(_) => unreachable!(),
        }
    }

    pub(crate) fn convert_value_to_children(&mut self, key: Box<[u8]>) {
        debug_assert!(
            core::mem::discriminant(&self.body)
                == core::mem::discriminant(&NodeBody::Value(T::default()))
        );

        let mut body = NodeBody::Children(vec![]);
        core::mem::swap(&mut body, &mut self.body);
        self.key = self.key[..self.key.len() - key.len()]
            .to_vec()
            .into_boxed_slice();
        self.push(Node { key, body });
    }

    pub fn insert(&mut self, index: usize, value: Node<T>) {
        match &mut self.body {
            NodeBody::Children(children) => {
                children.insert(index, value);
            }
            NodeBody::Value(_) => {
                self.convert_value_to_children(Default::default());
                self.push(value);
            }
        };
    }

    pub fn push(&mut self, value: Node<T>) {
        match &mut self.body {
            NodeBody::Children(children) => {
                children.push(value);
                children.sort_unstable();
            }
            NodeBody::Value(_value) => {
                self.convert_value_to_children(Default::default());
                self.push(value);
            }
        };
    }

    pub fn get_mut(&mut self, index: usize) -> &mut Node<T> {
        if let NodeBody::Value(_) = self.body {
            self.convert_value_to_children(Default::default());
            return self.get_mut(index);
        }

        match &mut self.body {
            NodeBody::Children(children) => &mut children[index],
            _ => unreachable!(),
        }
    }

    pub(crate) fn set_value(&mut self, value: T) {
        match &mut self.body {
            NodeBody::Children(_children) => panic!("set_value misused!"),
            NodeBody::Value(old_value) => {
                *old_value = value;
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl<T: Integer> Eq for Node<T> {}

#[cfg(feature = "alloc")]
impl<T: Integer> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

pub(crate) fn cmp(a: &[u8], b: &[u8]) -> Ordering {
    match b.len().cmp(&a.len()) {
        Ordering::Equal => a.cmp(b),
        x => x,
    }
}

#[cfg(feature = "alloc")]
impl<T: Integer> PartialOrd for Node<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(Ord::cmp(self, &other))
    }
}

#[cfg(feature = "alloc")]
impl<T: Integer> Ord for Node<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        cmp(&*self.key, &*other.key)
    }
}
