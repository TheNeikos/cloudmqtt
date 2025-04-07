//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;
use winnow::error::ParserError;

use crate::v5::MResult;
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WriteMqttPacket;

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
    packet_type: MUnsuback,
    anker: "_Toc3901190",
    pub struct UnsubackProperties<'i> {
        (anker: "_Toc3901192")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901193")
        user_properties: UserProperties<'i>,
    }
}

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901187")]
pub struct MUnsuback<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: UnsubackProperties<'i>,
    pub reasons: &'i [UnsubackReasonCode],
}

impl<'i> MUnsuback<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MUnsuback", |input: &mut &'i Bytes| {
            let packet_identifier = PacketIdentifier::parse(input)?;
            let properties = UnsubackProperties::parse(input)?;

            // Verify that the payload only contains valid reason codes
            let payload: &[u8] = winnow::combinator::repeat_till::<_, _, (), _, _, _, _>(
                0..,
                UnsubackReasonCode::parse,
                winnow::combinator::eof,
            )
            .take()
            .parse_next(input)?;

            let reasons: &[UnsubackReasonCode] = bytemuck::checked::try_cast_slice(payload)
                .map_err(|_e| {
                    winnow::error::ErrMode::Cut(winnow::error::ContextError::from_input(input))
                })?;

            Ok(Self {
                packet_identifier,
                properties,
                reasons,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.packet_identifier.binary_size()
            + self.reasons.len() as u32
            + self.properties.binary_size()
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> Result<(), W::Error> {
        self.packet_identifier.write(buffer)?;
        self.properties.write(buffer)?;

        let reasons: &[u8] = bytemuck::cast_slice(self.reasons);

        buffer.write_slice(reasons)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::unsuback::MUnsuback;
    use crate::v5::packets::unsuback::UnsubackProperties;
    use crate::v5::packets::unsuback::UnsubackReasonCode;
    use crate::v5::variable_header::PacketIdentifier;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_unsuback_no_props() {
        crate::v5::test::make_roundtrip_test!(MUnsuback {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(89).unwrap()),
            properties: UnsubackProperties {
                reason_string: None,
                user_properties: None
            },
            reasons: &[UnsubackReasonCode::Success],
        });
    }

    #[test]
    fn test_roundtrip_unsuback_props() {
        crate::v5::test::make_roundtrip_test!(MUnsuback {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(89).unwrap()),
            properties: UnsubackProperties {
                reason_string: Some(ReasonString("2345678")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            },
            reasons: &[UnsubackReasonCode::Success],
        });
    }
}
