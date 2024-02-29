//! Crate prelude

pub use crate::error::Error;
pub use crate::*;

pub type Result<T> = core::result::Result<T, Error>;

// Dependancies

#[doc(hidden)]
pub use serde::Serialize;
pub use clap::{value_parser, Arg, Command};
pub use std::fs::File;
pub use std::io;
pub use std::io::prelude::*;
pub use std::io::Read;
pub use std::io::SeekFrom;
pub use std::iter::Peekable;
pub use std::path::PathBuf;
