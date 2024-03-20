use winnow::Bytes;

use crate::v5::{
    fixed_header::PacketType,
    variable_header::{PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum PubrecReasonCode {
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NoMatchingSubscribers = crate::v5::reason_code::NoMatchingSubscribers,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        PacketIdentifierNotFound = crate::v5::reason_code::PacketIdentifierNotFound,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        Success = crate::v5::reason_code::Success,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
    }
}

crate::v5::properties::define_properties![
    pub struct PubrecProperties<'i> {
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
];

pub struct MPubrec<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubrecReasonCode,
    pub properties: PubrecProperties<'i>,
}

impl<'i> MPubrec<'i> {
    pub const PACKET_TYPE: PacketType = PacketType::Pubrec;

    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = PacketIdentifier::parse(input)?;
        let reason = PubrecReasonCode::parse(input)?;
        let properties = PubrecProperties::parse(input)?;
        Ok(Self {
            packet_identifier,
            reason,
            properties,
        })
    }
}
