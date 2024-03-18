mod integers;
mod strings;
mod bytes;
mod header;

pub type MResult<O> = winnow::PResult<O>;
