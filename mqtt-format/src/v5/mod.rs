pub mod bytes;
pub mod fixed_header;
pub mod integers;
pub mod reason_code;
pub mod strings;
pub mod variable_header;

pub type MResult<O> = winnow::PResult<O>;
