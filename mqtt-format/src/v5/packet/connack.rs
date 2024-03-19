use winnow::{
    error::{ErrMode, InputError, ParserError},
    Bytes, Parser,
};

use crate::v5::{fixed_header::PacketType, MResult};

#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum ConnectReasonCode {
    Success = 0,
    UnspecifiedError = 0x80,
    MalformedPacket = 0x81,
    ProtocolError = 0x82,
    ImplementationSpecificError = 0x84,
    ClientIdentifierNotValid = 0x85,
    BadUsernameOrPassword = 0x86,
    NotAuthorized = 0x87,
    ServerUnavailable = 0x88,
    ServerBusy = 0x89,
    Banned = 0x8A,
    BadAuthenticationMethod = 0x8C,
    TopicNameInvalid = 0x90,
    PacketTooLarge = 0x95,
    QuotaExceeded = 0x97,
    PayloadFormatInvalid = 0x99,
    RetainNotSupported = 0x9A,
    QoSNotSupported = 0x9B,
    UseAnotherServer = 0x9C,
    ServerMoved = 0x9D,
    ConnectionRateExceeded = 0x9F,
}

impl ConnectReasonCode {
    fn parse<'i>(input: &mut &'i Bytes) -> MResult<ConnectReasonCode> {
        winnow::binary::u8
            .try_map(ConnectReasonCode::try_from)
            .parse_next(input)
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
