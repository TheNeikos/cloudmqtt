//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;

use crate::v5::MResult;
use crate::v5::properties::define_properties;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::ServerReference;
use crate::v5::variable_header::SessionExpiryInterval;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;

crate::v5::reason_code::make_combined_reason_code! {
    pub enum DisconnectReasonCode {
        AdministrativeAction = crate::v5::reason_code::AdministrativeAction,
        ConnectionRateExceeded = crate::v5::reason_code::ConnectionRateExceeded,
        DisconnectWithWillMessage = crate::v5::reason_code::DisconnectWithWillMessage,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        KeepAliveTimeout = crate::v5::reason_code::KeepAliveTimeout,
        MalformedPacket = crate::v5::reason_code::MalformedPacket,
        MaximumConnectTime = crate::v5::reason_code::MaximumConnectTime,
        MessageRateTooHigh = crate::v5::reason_code::MessageRateTooHigh,
        NormalDisconnection = crate::v5::reason_code::NormalDisconnection,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        PacketTooLarge = crate::v5::reason_code::PacketTooLarge,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        ProtocolError = crate::v5::reason_code::ProtocolError,
        QoSNotSupported = crate::v5::reason_code::QoSNotSupported,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        ReceiveMaximumExceeded = crate::v5::reason_code::ReceiveMaximumExceeded,
        RetainNotSupported = crate::v5::reason_code::RetainNotSupported,
        ServerBusy = crate::v5::reason_code::ServerBusy,
        ServerMoved = crate::v5::reason_code::ServerMoved,
        ServerShuttingDown = crate::v5::reason_code::ServerShuttingDown,
        SessionTakenOver = crate::v5::reason_code::SessionTakenOver,
        SharedSubscriptionsNotSupported = crate::v5::reason_code::SharedSubscriptionsNotSupported,
        SubscriptionIdentifiersNotSupported = crate::v5::reason_code::SubscriptionIdentifiersNotSupported,
        TopicAliasInvalid = crate::v5::reason_code::TopicAliasInvalid,
        TopicFilterInvalid = crate::v5::reason_code::TopicFilterInvalid,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        UseAnotherServer = crate::v5::reason_code::UseAnotherServer,
        WildcardSubscriptionsNotSupported = crate::v5::reason_code::WildcardSubscriptionsNotSupported,
    }
}

define_properties! {
    packet_type: MDisconnect,
    anker: "_Toc3901209",
    pub struct DisconnectProperties<'i> {
        (anker: "_Toc3901211")
        session_expiry_interval: SessionExpiryInterval,

        (anker: "_Toc3901212")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901213")
        user_properties: UserProperties<'i>,

        (anker: "_Toc3901214")
        server_reference: ServerReference<'i>
    }
}

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901205")]
pub struct MDisconnect<'i> {
    pub reason_code: DisconnectReasonCode,
    pub properties: DisconnectProperties<'i>,
}

impl<'i> MDisconnect<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MDisconnect<'i>> {
        winnow::combinator::trace("MDisconnect", |input: &mut &'i Bytes| {
            // The Reason Code and Property Length can be omitted if the Reason Code is 0x00 (Normal disconnecton)
            // and there are no Properties. In this case the DISCONNECT has a Remaining Length of 0.
            let reason_code = if input.is_empty() {
                DisconnectReasonCode::NormalDisconnection
            } else {
                DisconnectReasonCode::parse(input)?
            };
            let properties = if input.is_empty() {
                DisconnectProperties::new()
            } else {
                DisconnectProperties::parse(input)?
            };

            Ok(MDisconnect {
                reason_code,
                properties,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        if self.is_short_packet() {
            return 0;
        }
        self.reason_code.binary_size() + self.properties.binary_size()
    }
    #[inline]
    fn is_short_packet(&self) -> bool {
        // if reason code is NormalDisconnection AND properties are empty, we can skip writing the payload
        self.reason_code == DisconnectReasonCode::NormalDisconnection
            && self.properties == DisconnectProperties::new()
    }
    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        if self.is_short_packet() {
            return Ok(());
        }
        self.reason_code.write(buffer)?;
        self.properties.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use super::DisconnectProperties;
    use super::MDisconnect;
    use crate::v5::packets::MqttPacket;
    use crate::v5::packets::disconnect::DisconnectReasonCode;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::ServerReference;
    use crate::v5::variable_header::SessionExpiryInterval;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_disconnect_no_props() {
        crate::v5::test::make_roundtrip_test!(MDisconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties {
                session_expiry_interval: None,
                reason_string: None,
                user_properties: None,
                server_reference: None,
            },
        });
    }

    #[test]
    fn test_roundtrip_disconnect_with_props() {
        crate::v5::test::make_roundtrip_test!(MDisconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties {
                session_expiry_interval: Some(SessionExpiryInterval(123)),
                reason_string: Some(ReasonString("foo")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
                server_reference: Some(ServerReference("barbarbar")),
            },
        });
    }

    #[test]
    fn test_short_disconnect_packet() {
        // handle special case https://github.com/TheNeikos/cloudmqtt/issues/291
        let buf = [0xe0, 0x00];
        let parsed = MqttPacket::parse_complete(&buf).unwrap();
        let reference = MqttPacket::Disconnect(MDisconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::new(),
        });
        assert_eq!(parsed, reference);
    }

    #[test]
    fn test_short_disconnect_encoding() {
        let reference = MqttPacket::Disconnect(MDisconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties::new(),
        });
        let mut writer = crate::v5::test::TestWriter { buffer: Vec::new() };
        reference.write(&mut writer).unwrap();
        let buf = [0xe0, 0x00];
        assert_eq!(writer.buffer, buf);
    }

    #[test]
    fn test_short_disconnect_long_encoding() {
        let reference = MqttPacket::Disconnect(MDisconnect {
            reason_code: DisconnectReasonCode::NormalDisconnection,
            properties: DisconnectProperties {
                session_expiry_interval: Some(SessionExpiryInterval(123)),
                reason_string: None,
                user_properties: None,
                server_reference: None,
            },
        });
        let mut writer = crate::v5::test::TestWriter { buffer: Vec::new() };
        reference.write(&mut writer).unwrap();
        assert!(
            writer.buffer.len() > 2,
            "the extra rule for short NormalDisconnect should not influence more complex disconnects"
        );
    }
}
