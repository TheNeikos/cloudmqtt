pub mod bytes;
pub mod fixed_header;
pub mod variable_header;
pub mod integers;
pub mod strings;

pub type MResult<O> = winnow::PResult<O>;
