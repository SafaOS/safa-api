# safa-api
An API that exposes [SafaOS's](https://github.com/SafaOS/SafaOS) syscalls, and provides a higher-level interface over some syscalls.

[![crates.io](https://img.shields.io/crates/v/safa-api)](https://crates.io/crates/safa-api)
[![docs.rs](https://img.shields.io/docsrs/safa-api)](https://docs.rs/safa-api)

## Usage
if you want to use this crate in your rust project
simply run
```
cargo add safa-api
```
if you have std then you can also add the feature `std`
```
cargo add safa-api --features std
```

if you want to use it in any other project which is written in a language that has a C FFi
(such as C, C++,  etc.)
compile the library as a static library (**TODO: add instructions for how to do this**)
and then link it to your project