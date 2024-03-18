pub mod bytes;
pub mod header;
pub mod integers;
pub mod strings;

pub type MResult<O> = winnow::PResult<O>;
