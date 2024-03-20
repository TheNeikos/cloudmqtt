use winnow::Bytes;

use crate::v5::{
    fixed_header::PacketType,
    properties::define_properties,
    variable_header::{PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum PubackReasonCode {
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NoMatchingSubscribers = crate::v5::reason_code::NoMatchingSubscribers,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        Success = crate::v5::reason_code::Success,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
    }
}

define_properties!(
    packet_type: MPuback,
    anker: "_Toc3901125",
    pub struct PubackProperties<'i> {
        (anker: "_Toc3901127")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901128")
        user_properties: UserProperties<'i>,
    }
);

pub struct MPuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubackReasonCode,
    pub properties: PubackProperties<'i>,
}

impl<'i> MPuback<'i> {
    pub const PACKET_TYPE: PacketType = PacketType::Puback;

    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = PacketIdentifier::parse(input)?;

        if input.is_empty() {
            Ok(Self {
                packet_identifier,
                reason: PubackReasonCode::Success,
                properties: PubackProperties::new(),
            })
        } else {
            let reason = PubackReasonCode::parse(input)?;
            let properties = PubackProperties::parse(input)?;
            Ok(Self {
                packet_identifier,
                reason,
                properties,
            })
        }
    }
}
