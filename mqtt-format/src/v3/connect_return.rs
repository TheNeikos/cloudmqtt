//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use nom::error::FromExternalError;

use super::errors::MPacketHeaderError;
use super::MSResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MConnectReturnCode {
    Accepted = 0x0,
    ProtocolNotAccepted = 0x1,
    IdentifierRejected = 0x2,
    ServerUnavailable = 0x3,
    BadUsernamePassword = 0x4,
    NotAuthorized = 0x5,
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
