use winnow::Bytes;

use crate::v5::{
    variable_header::{PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum PubcompReasonCode {
        PacketIdentifierNotFound = crate::v5::reason_code::PacketIdentifierNotFound,
        Success = crate::v5::reason_code::Success,
    }
}

crate::v5::properties::define_properties! {
    pub struct PubcompProperties <'i> {
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
}

pub struct MPubcomp<'i> {
    packet_identifier: PacketIdentifier,
    reason: PubcompReasonCode,
    properties: PubcompProperties<'i>,
}

impl<'i> MPubcomp<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = PacketIdentifier::parse(input)?;
        let reason = PubcompReasonCode::parse(input)?;
        let properties = PubcompProperties::parse(input)?;
        Ok(Self {
            packet_identifier,
            reason,
            properties,
        })
    }
}
