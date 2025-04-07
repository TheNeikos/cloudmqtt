//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::MResult;
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WriteMqttPacket;

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

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901151")]
pub struct MPubcomp<'i> {
    pub packet_identifier: PacketIdentifier,
    pub reason: PubcompReasonCode,
    pub properties: PubcompProperties<'i>,
}

impl<'i> MPubcomp<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MPubcomp", |input: &mut &'i Bytes| {
            let packet_identifier = PacketIdentifier::parse(input)?;

            let reason = if input.is_empty() {
                PubcompReasonCode::Success
            } else {
                PubcompReasonCode::parse(input)?
            };

            let properties = if input.is_empty() {
                PubcompProperties::new()
            } else {
                PubcompProperties::parse(input)?
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

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> Result<(), W::Error> {
        self.packet_identifier.write(buffer)?;
        self.reason.write(buffer)?;
        self.properties.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::pubcomp::MPubcomp;
    use crate::v5::packets::pubcomp::PubcompProperties;
    use crate::v5::packets::pubcomp::PubcompReasonCode;
    use crate::v5::variable_header::PacketIdentifier;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_pubcomp_no_props() {
        crate::v5::test::make_roundtrip_test!(MPubcomp {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(123).unwrap()),
            reason: PubcompReasonCode::Success,
            properties: PubcompProperties {
                reason_string: None,
                user_properties: None,
            },
        });
    }

    #[test]
    fn test_roundtrip_puback_with_props() {
        crate::v5::test::make_roundtrip_test!(MPubcomp {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(123).unwrap()),
            reason: PubcompReasonCode::Success,
            properties: PubcompProperties {
                reason_string: Some(ReasonString("fooooo")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            }
        });
    }
}
