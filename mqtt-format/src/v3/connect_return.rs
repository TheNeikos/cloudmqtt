use nom::error::FromExternalError;

use super::{errors::MPacketHeaderError, MSResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MConnectReturnCode {
    Accepted,
    ProtocolNotAccepted,
    IdentifierRejected,
    ServerUnavailable,
    BadUsernamePassword,
    NotAuthorized,
}

pub fn mconnectreturn(input: &[u8]) -> MSResult<'_, MConnectReturnCode> {
    let (input, return_code) = nom::number::complete::u8(input)?;

    Ok((
        input,
        match return_code {
            0 => MConnectReturnCode::Accepted,
            1 => MConnectReturnCode::ProtocolNotAccepted,
            2 => MConnectReturnCode::IdentifierRejected,
            3 => MConnectReturnCode::ServerUnavailable,
            4 => MConnectReturnCode::BadUsernamePassword,
            5 => MConnectReturnCode::NotAuthorized,
            invalid_code => {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InvalidConnectReturnCode(invalid_code),
                )))
            }
        },
    ))
}
