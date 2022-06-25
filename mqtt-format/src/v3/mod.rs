use nom::IResult;

pub mod errors;
pub mod header;
pub mod identifier;
pub mod packet;
pub mod qos;
pub mod strings;
pub mod will;
pub mod connect_return;
pub mod subscription_request;

/// The result of a streaming operation
pub type MSResult<'a, T> = IResult<&'a [u8], T>;
