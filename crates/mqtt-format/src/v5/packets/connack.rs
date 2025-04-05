//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::Parser;
use winnow::error::ErrMode;
use winnow::error::InputError;
use winnow::error::ParserError;

use crate::v5::MResult;
use crate::v5::properties::define_properties;
use crate::v5::variable_header::AssignedClientIdentifier;
use crate::v5::variable_header::AuthenticationData;
use crate::v5::variable_header::AuthenticationMethod;
use crate::v5::variable_header::MaximumPacketSize;
use crate::v5::variable_header::MaximumQoS;
use crate::v5::variable_header::ReasonString;
use crate::v5::variable_header::ReceiveMaximum;
use crate::v5::variable_header::ResponseInformation;
use crate::v5::variable_header::RetainAvailable;
use crate::v5::variable_header::ServerKeepAlive;
use crate::v5::variable_header::ServerReference;
use crate::v5::variable_header::SessionExpiryInterval;
use crate::v5::variable_header::SharedSubscriptionAvailable;
use crate::v5::variable_header::SubscriptionIdentifiersAvailable;
use crate::v5::variable_header::TopicAliasMaximum;
use crate::v5::variable_header::UserProperties;
use crate::v5::variable_header::WildcardSubscriptionAvailable;
use crate::v5::write::WriteMqttPacket;

crate::v5::reason_code::make_combined_reason_code! {
    pub enum ConnackReasonCode {
        Success = crate::v5::reason_code::Success,
        UnspecifiedError = crate::v5::reason_code::UnspecifiedError,
        MalformedPacket = crate::v5::reason_code::MalformedPacket,
        ProtocolError = crate::v5::reason_code::ProtocolError,
        ImplementationSpecificError = crate::v5::reason_code::ImplementationSpecificError,
        ClientIdentifierNotValid = crate::v5::reason_code::ClientIdentifierNotValid,
        BadUsernameOrPassword = crate::v5::reason_code::BadUsernameOrPassword,
        NotAuthorized = crate::v5::reason_code::NotAuthorized,
        ServerUnavailable = crate::v5::reason_code::ServerUnavailable,
        ServerBusy = crate::v5::reason_code::ServerBusy,
        Banned = crate::v5::reason_code::Banned,
        BadAuthenticationMethod = crate::v5::reason_code::BadAuthenticationMethod,
        TopicNameInvalid = crate::v5::reason_code::TopicNameInvalid,
        PacketTooLarge = crate::v5::reason_code::PacketTooLarge,
        QuotaExceeded = crate::v5::reason_code::QuotaExceeded,
        PayloadFormatInvalid = crate::v5::reason_code::PayloadFormatInvalid,
        RetainNotSupported = crate::v5::reason_code::RetainNotSupported,
        QoSNotSupported = crate::v5::reason_code::QoSNotSupported,
        UseAnotherServer = crate::v5::reason_code::UseAnotherServer,
        ServerMoved = crate::v5::reason_code::ServerMoved,
        ConnectionRateExceeded = crate::v5::reason_code::ConnectionRateExceeded,
    }
}

define_properties![
    packet_type: MConnack,
    anker: "_Toc3901080",
    pub struct ConnackProperties<'i> {
        (anker: "_Toc3901082")
        session_expiry_interval: SessionExpiryInterval,

        (anker: "_Toc3901083")
        receive_maximum: ReceiveMaximum,

        (anker: "_Toc3901084")
        maximum_qos: MaximumQoS,

        (anker: "_Toc3901085")
        retain_available: RetainAvailable,

        (anker: "_Toc3901086")
        maximum_packet_size: MaximumPacketSize,

        (anker: "_Toc3901087")
        assigned_client_identifier: AssignedClientIdentifier<'i>,

        (anker: "_Toc3901088")
        topic_alias_maximum: TopicAliasMaximum,

        (anker: "_Toc3901089")
        reason_string: ReasonString<'i>,

        (anker: "_Toc3901090")
        user_properties: UserProperties<'i>,

        (anker: "_Toc3901091")
        wildcard_subscription_available: WildcardSubscriptionAvailable,

        (anker: "_Toc3901092")
        subscription_identifiers_available: SubscriptionIdentifiersAvailable,

        (anker: "_Toc3901093")
        shared_scubscription_available: SharedSubscriptionAvailable,

        (anker: "_Toc3901094")
        server_keep_alive: ServerKeepAlive,

        (anker: "_Toc3901095")
        response_information: ResponseInformation<'i>,

        (anker: "_Toc3901096")
        server_reference: ServerReference<'i>,

        (anker: "_Toc3901097")
        authentication_method: AuthenticationMethod<'i>,

        (anker: "_Toc3901098")
        authentication_data: AuthenticationData<'i>,
    }
];

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901074")]
pub struct MConnack<'i> {
    pub session_present: bool,
    pub reason_code: ConnackReasonCode,
    pub properties: ConnackProperties<'i>,
}

impl<'i> MConnack<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MConnack<'i>> {
        winnow::combinator::trace("MConnack", |input: &mut &'i Bytes| {
            let (session_present, _) =
                winnow::binary::bits::bits::<_, _, ErrMode<InputError<(_, usize)>>, _, _>((
                    winnow::binary::bits::take(1usize).map(|b: u8| b == 1),
                    winnow::binary::bits::pattern(0b000_0000, 7usize),
                ))
                .parse_next(input)
                .map_err(|_: ErrMode<InputError<_>>| ErrMode::from_input(input))?;

            let reason_code = ConnackReasonCode::parse(input)?;
            let properties = ConnackProperties::parse(input)?;

            Ok(MConnack {
                session_present,
                reason_code,
                properties,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        1 // flags
        + self.reason_code.binary_size()
        + self.properties.binary_size()
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> Result<(), W::Error> {
        let byte = (self.session_present as u8) << 7;
        buffer.write_byte(byte)?;
        self.reason_code.write(buffer)?;
        self.properties.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use super::ConnackProperties;
    use super::ConnackReasonCode;
    use super::MConnack;
    use crate::v5::variable_header::AssignedClientIdentifier;
    use crate::v5::variable_header::AuthenticationData;
    use crate::v5::variable_header::AuthenticationMethod;
    use crate::v5::variable_header::MaximumPacketSize;
    use crate::v5::variable_header::MaximumQoS;
    use crate::v5::variable_header::ReasonString;
    use crate::v5::variable_header::ReceiveMaximum;
    use crate::v5::variable_header::ResponseInformation;
    use crate::v5::variable_header::RetainAvailable;
    use crate::v5::variable_header::ServerKeepAlive;
    use crate::v5::variable_header::ServerReference;
    use crate::v5::variable_header::SessionExpiryInterval;
    use crate::v5::variable_header::SharedSubscriptionAvailable;
    use crate::v5::variable_header::SubscriptionIdentifiersAvailable;
    use crate::v5::variable_header::TopicAliasMaximum;
    use crate::v5::variable_header::UserProperties;
    use crate::v5::variable_header::WildcardSubscriptionAvailable;

    #[test]
    fn test_roundtrip_connack_no_props() {
        crate::v5::test::make_roundtrip_test!(MConnack {
            session_present: true,
            reason_code: ConnackReasonCode::Success,
            properties: ConnackProperties {
                session_expiry_interval: None,
                receive_maximum: None,
                maximum_qos: None,
                retain_available: None,
                maximum_packet_size: None,
                assigned_client_identifier: None,
                topic_alias_maximum: None,
                reason_string: None,
                user_properties: None,
                wildcard_subscription_available: None,
                subscription_identifiers_available: None,
                shared_scubscription_available: None,
                server_keep_alive: None,
                response_information: None,
                server_reference: None,
                authentication_method: None,
                authentication_data: None,
            }
        });
    }

    #[test]
    fn test_roundtrip_connack_with_props() {
        crate::v5::test::make_roundtrip_test!(MConnack {
            session_present: true,
            reason_code: ConnackReasonCode::Success,
            properties: ConnackProperties {
                session_expiry_interval: Some(SessionExpiryInterval(120)),
                receive_maximum: Some(ReceiveMaximum(core::num::NonZeroU16::new(123).unwrap())),
                maximum_qos: Some(MaximumQoS(
                    crate::v5::qos::MaximumQualityOfService::AtMostOnce
                )),
                retain_available: Some(RetainAvailable(true)),
                maximum_packet_size: Some(MaximumPacketSize(1024)),
                assigned_client_identifier: Some(AssignedClientIdentifier("foobar")),
                topic_alias_maximum: Some(TopicAliasMaximum(1234)),
                reason_string: Some(ReasonString("reason")),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
                wildcard_subscription_available: Some(WildcardSubscriptionAvailable(123)),
                subscription_identifiers_available: Some(SubscriptionIdentifiersAvailable(123)),
                shared_scubscription_available: Some(SharedSubscriptionAvailable(123)),
                server_keep_alive: Some(ServerKeepAlive(123)),
                response_information: Some(ResponseInformation("fofofo")),
                server_reference: Some(ServerReference("barbarbar")),
                authentication_method: Some(AuthenticationMethod("bazbazbaz")),
                authentication_data: Some(AuthenticationData(&[0xFF, 0xFF])),
            }
        });
    }
}
