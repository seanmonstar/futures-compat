#![deny(warnings)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! # futures-compat
//!
//! A compatibility layer between different versions of [Future][futures].
//!
//! [futures]: https://crates.io/crates/futures

extern crate futures;
extern crate futures_core;

pub mod futures_01;
pub mod futures_02;
