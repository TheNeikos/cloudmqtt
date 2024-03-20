//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::MResult;

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
    packet_type: MSuback,
    anker: "_Toc3901174",
    pub struct SubackProperties<'i> {
        (anker: "_Toc3901175")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901176")
        user_properties: UserProperties<'i>,
    }
}

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901171")]
pub struct MSuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: SubackProperties<'i>,
    pub reasons: &'i [SubackReasonCode],
}

impl<'i> MSuback<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MSuback", |input: &mut &'i Bytes| {
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
            let reasons: &[SubackReasonCode] = unsafe { core::mem::transmute(payload) };

            Ok(Self {
                packet_identifier,
                properties,
                reasons,
            })
        })
        .parse_next(input)
    }
}
