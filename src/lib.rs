#![feature(range_contains)]
#![feature(rustc_private)]
#![feature(conservative_impl_trait)]
#![feature(plugin)]
#![cfg_attr(test, plugin(stainless))]
#![feature(const_fn)]
#![feature(try_trait)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate generic_array;
#[allow(unused_imports)]
#[macro_use]
extern crate nom;

#[macro_use]
pub mod helpers;
pub mod lexeme_scanner;
pub mod syntax_parser;
