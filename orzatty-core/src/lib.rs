#![cfg_attr(not(feature = "std"), no_std)]

//! # Orzatty Core
//! 
//! Core definitions for the Protocolo Orzatty wire format.
//! This crate is `no_std` compatible by default to ensure portability to WASM and embedded targets.
//! Enable the `std` feature for framer support.

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod frame;
pub mod protocol;
pub mod error;
pub mod auth;

#[cfg(feature = "quinn")]
pub mod framer;

pub use frame::{FrameHeader, FrameType, FrameFlags};
pub use error::Error;

#[cfg(feature = "quinn")]
pub use framer::Framer;
