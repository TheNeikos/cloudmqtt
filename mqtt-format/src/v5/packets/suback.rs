use winnow::{Bytes, Parser};

use crate::v5::{
    variable_header::{PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum SubackReasonCode {
        GrantedQoS0 = crate::v5::reason_code::GrantedQoS0,
        GrantedQoS1 = crate::v5::reason_code::GrantedQoS1,
        GrantedQoS2 = crate::v5::reason_code::GrantedQoS2,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        SharedSubscriptionsNotSupported = crate::v5::reason_code::SharedSubscriptionsNotSupported,
        SubscriptionIdentifiersNotSupported = crate::v5::reason_code::SubscriptionIdentifiersNotSupported,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        WildcardSubscriptionsNotSupported = crate::v5::reason_code::WildcardSubscriptionsNotSupported,
    }
}

crate::v5::properties::define_properties! {
    pub struct SubackProperties<'i> {
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
}

pub struct MSuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: SubackProperties<'i>,
    pub reasons: &'i [SubackReasonCode],
}

impl<'i> MSuback<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = PacketIdentifier::parse(input)?;
        let properties = SubackProperties::parse(input)?;

        // Verify that the payload only contains valid reason codes
        let payload: &[u8] = winnow::combinator::repeat_till::<_, _, (), _, _, _, _>(
            0..,
            SubackReasonCode::parse,
            winnow::combinator::eof,
        )
        .recognize()
        .parse_next(input)?;

        // SAFETY: We verified above that the payload slice only contains valid SubackReasonCode
        // bytes
        let reasons: &[SubackReasonCode] = unsafe { std::mem::transmute(payload) };

        Ok(Self {
            packet_identifier,
            properties,
            reasons,
        })
    }
}
