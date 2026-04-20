//! # gyres-polar
//!
//! Embedded policy engine for Gyres permission evaluation.
//! Forked and trimmed from the Oso Polar engine — keeps the parser,
//! evaluator, and Rust type registration; removes FFI bindings,
//! ORM adapters, and cloud client.

#![forbid(unsafe_code)]
