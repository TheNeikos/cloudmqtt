#![deny(missing_debug_implementations)]

pub mod bytes;
pub mod fixed_header;
pub mod integers;
pub mod level;
pub mod packets;
pub mod properties;
pub mod reason_code;
pub mod strings;
pub mod util;
pub mod variable_header;

pub type MResult<O> = winnow::PResult<O>;
