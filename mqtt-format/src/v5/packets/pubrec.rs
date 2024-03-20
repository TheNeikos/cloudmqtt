//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;

use crate::v5::{
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
    packet_type: MPubrec,
    anker: "_Toc3901135",
    pub struct PubrecProperties<'i> {
        (anker: "_Toc3901137")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901138")
        user_properties: UserProperties<'i>,
    }
];

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901131")]
pub struct MPubrec<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubrecReasonCode,
    pub properties: PubrecProperties<'i>,
}

impl<'i> MPubrec<'i> {
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
