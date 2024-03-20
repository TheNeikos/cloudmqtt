use winnow::{
    error::{ErrMode, InputError, ParserError},
    Bytes, Parser,
};

use crate::v5::{fixed_header::PacketType, MResult};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum ConnectReasonCode {
        Success = crate::v5::reason_code::Success,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        MalformedPacket = crate::v5::reason_code::MalformedPacket,
        ProtocolError = crate::v5::reason_code::ProtocolError,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        ClientIdentifierNotValid = crate::v5::reason_code::ClientIdentifierNotValid,
        BadUsernameOrPassword = crate::v5::reason_code::BadUsernameOrPassword,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        ServerUnavailable = crate::v5::reason_code::ServerUnavailable,
        ServerBusy = crate::v5::reason_code::ServerBusy,
        Banned = crate::v5::reason_code::Banned,
        BadAuthenticationMethod = crate::v5::reason_code::BadAuthenticationMethod,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        PacketTooLarge = crate::v5::reason_code::PacketTooLarge,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        RetainNotSupported = crate::v5::reason_code::RetainNotSupported,
        QoSNotSupported = crate::v5::reason_code::QoSNotSupported,
        UseAnotherServer = crate::v5::reason_code::UseAnotherServer,
        ServerMoved = crate::v5::reason_code::ServerMoved,
        ConnectionRateExceeded = crate::v5::reason_code::ConnectionRateExceeded,
    }
}

pub struct MConnack<'i> {
    pub session_present: bool,
    pub reason_code: ConnectReasonCode,
    pd: &'i (),
}

impl<'i> MConnack<'i> {
    pub const PACKET_TYPE: PacketType = PacketType::Connack;

    pub fn parse(input: &mut &'i Bytes) -> MResult<MConnack<'i>> {
        let (session_present, _) =
            winnow::binary::bits::bits::<_, _, InputError<(_, usize)>, _, _>((
                winnow::binary::bits::take(1usize).map(|b: u8| b == 1),
                winnow::binary::bits::pattern(0b0000_000, 7usize),
            ))
            .parse_next(input)
            .map_err(|_: ErrMode<InputError<_>>| {
                ErrMode::from_error_kind(input, winnow::error::ErrorKind::Slice)
            })?;

        let reason_code = ConnectReasonCode::parse(input)?;

        Ok(MConnack {
            session_present,
            reason_code,
            pd: &(),
        })
    }
}
