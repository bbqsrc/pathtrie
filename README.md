# pathtrie

[![Documentation](https://docs.rs/pathtrie/badge.svg)](https://docs.rs/pathtrie)

A specialised trie for paths in the style of a Patricia or radix trie, with optional optimised FST output.

The intended usage of this data structure is for optimally storing and querying keys that have a large number of shared prefixes, such as file paths in a file system.

This crate is partly inspired by the [`fst` crate by Andrew Gallant](https://github.com/BurntSushi/fst). There are a few significant differences to that crate, however:

- Simplicity of implementation was prioritised over speed
- The trie structure is mutable and can be later written into an FST
- Insertions do not need to be in lexicographical order

It is a goal of this project to stabilise the FST format once proven to be bug-free.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
pathtrie = "0.1"
```

## Where is this used?

* [box](https://github.com/bbqsrc/box) - a modern replacement for the zip file format

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
