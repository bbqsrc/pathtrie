

use std::num::NonZeroU32;

struct Fst {
    header: Header,
    record_count: u32,

}

#[repr(C)]
struct Header {
    magic_bytes: [u8; 2], // \xff, \xdf
    version: u8, // 0
    alignment: u8, // ie, are our offsets 2-byte, 4-byte or 8-byte aligned
}

#[repr(transparent)]
struct FileOffset(pub(crate) Option<NonZeroU32>);

#[repr(C)]
struct Node {
    value: FileOffset, // If null, there's no solution for this input.
    output: FileOffset, // as above.
}

#[repr(C)]
struct Value {
    next_node: FileOffset, // If null, value is the output terminal value
    value: [u8],
}

