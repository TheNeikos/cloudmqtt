//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;

use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::MResult;

crate::v5::reason_code::make_combined_reason_code! {
    pub enum PubcompReasonCode {
        PacketIdentifierNotFound = crate::v5::reason_code::PacketIdentifierNotFound,
        Success = crate::v5::reason_code::Success,
    }
}

crate::v5::properties::define_properties! {
    packet_type: MPubcomp,
    anker: "_Toc3901153",
    pub struct PubcompProperties <'i> {
        (anker: "_Toc3901154")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901155")
        user_properties: UserProperties<'i>,
    }
}

#[derive(Debug)]
#[doc = crate::v5::util::md_speclink!("_Toc3901151")]
pub struct MPubcomp<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubcompReasonCode,
    pub properties: PubcompProperties<'i>,
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
