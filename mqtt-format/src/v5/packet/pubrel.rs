use winnow::Bytes;

use crate::v5::{
    fixed_header::PacketType,
    properties::define_properties,
    variable_header::{parse_packet_identifier, PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum PubrelReasonCode {
        Success = crate::v5::reason_code::Success,
        PacketIdentifierNotFound = crate::v5::reason_code::PacketIdentifierNotFound,
    }
}

define_properties!(
    pub struct PubrelProperties<'i> {
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
);

pub struct MPubrel<'i> {
    packet_identifier: PacketIdentifier,
    reason: PubrelReasonCode,
    properties: PubrelProperties<'i>,
}

impl<'i> MPubrel<'i> {
    pub const PACKET_TYPE: PacketType = PacketType::Pubrel;

    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = parse_packet_identifier(input)?;

        if input.is_empty() {
            Ok(Self {
                packet_identifier,
                reason: PubrelReasonCode::Success,
                properties: PubrelProperties::new(),
            })
        } else {
            let reason = PubrelReasonCode::parse(input)?;
            let properties = PubrelProperties::parse(input)?;
            Ok(Self {
                packet_identifier,
                reason,
                properties,
            })
        }
    }
}
