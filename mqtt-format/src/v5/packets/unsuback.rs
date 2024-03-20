//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::{
    variable_header::{PacketIdentifier, ReasonString, UserProperties},
    MResult,
};

crate::v5::reason_code::make_combined_reason_code! {
    pub enum UnsubackReasonCode {
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        NoSubscriptionExisted = crate::v5::reason_code::NoSubscriptionExisted,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketIdentifierInUse = crate::v5::reason_code::PacketIdentifierInUse,
        Success = crate::v5::reason_code::Success,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
    }
}

crate::v5::properties::define_properties! {
    pub struct UnsubackProperties<'i> {
        reason_string: ReasonString<'i>,
        user_properties: UserProperties<'i>,
    }
}

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901187")]
pub struct MUnsuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: UnsubackProperties<'i>,
    pub reasons: &'i [UnsubackReasonCode],
}

impl<'i> MUnsuback<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let packet_identifier = PacketIdentifier::parse(input)?;
        let properties = UnsubackProperties::parse(input)?;

        // Verify that the payload only contains valid reason codes
        let payload: &[u8] = winnow::combinator::repeat_till::<_, _, (), _, _, _, _>(
            0..,
            UnsubackReasonCode::parse,
            winnow::combinator::eof,
        )
        .recognize()
        .parse_next(input)?;

        // SAFETY: We verified above that the payload slice only contains valid UnsubackReasonCode
        // bytes
        let reasons: &[UnsubackReasonCode] = unsafe { std::mem::transmute(payload) };

        Ok(Self {
            packet_identifier,
            properties,
            reasons,
        })
    }
}
