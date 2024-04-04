//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::properties::define_properties;
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

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

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901121")]
pub struct MPuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubackReasonCode,
    pub properties: PubackProperties<'i>,
}

impl<'i> MPuback<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MPuback", |input: &mut &'i Bytes| {
            let packet_identifier = PacketIdentifier::parse(input)?;

            let reason = if input.is_empty() {
                PubackReasonCode::Success
            } else {
                PubackReasonCode::parse(input)?
            };

            let properties = if input.is_empty() {
                PubackProperties::new()
            } else {
                PubackProperties::parse(input)?
            };

            Ok(Self {
                packet_identifier,
                reason,
                properties,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.packet_identifier.binary_size()
            + self.reason.binary_size()
            + self.properties.binary_size()
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        self.packet_identifier.write(buffer)?;
        self.reason.write(buffer)?;
        self.properties.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::puback::MPuback;
    use crate::v5::packets::puback::PubackProperties;
    use crate::v5::packets::puback::PubackReasonCode;
    use crate::v5::variable_header::PacketIdentifier;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_puback_no_props() {
        crate::v5::test::make_roundtrip_test!(MPuback {
            packet_identifier: PacketIdentifier(123),
            reason: PubackReasonCode::Success,
            properties: PubackProperties {
                reason_string: None,
                user_properties: None,
            }
        });
    }

    #[test]
    fn test_roundtrip_puback_with_props() {
        crate::v5::test::make_roundtrip_test!(MPuback {
            packet_identifier: PacketIdentifier(123),
            reason: PubackReasonCode::Success,
            properties: PubackProperties {
                reason_string: Some(ReasonString("fooooo")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            }
        });
    }
}
