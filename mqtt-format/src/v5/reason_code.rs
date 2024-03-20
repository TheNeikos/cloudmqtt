//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

macro_rules! make_combined_reason_code {
    (pub enum $name:ident {
        $($reason_code_name:ident = $reason_code_type:ty),* $(,)?
    }) => {
        #[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
        #[repr(u8)]
        #[derive(Debug, Copy, Clone)]
        pub enum $name {
            $( $reason_code_name = <$reason_code_type>::CODE ),*
        }

        impl $name {
            pub fn parse(input: &mut &winnow::Bytes) -> crate::v5::MResult<Self> {
                use winnow::Parser;
                winnow::binary::u8
                    .try_map($name::try_from)
                    .parse_next(input)
            }
        }
    }
}
pub(crate) use make_combined_reason_code;

macro_rules! define_reason_code {
    ($name:ident => $code:literal) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $name;
        impl $name {
            pub const CODE: u8 = $code;
        }
    };
}

define_reason_code!(GrantedQoS0 => 0x00);
define_reason_code!(NormalDisconnection => 0x00);
define_reason_code!(Success => 0x00);
define_reason_code!(GrantedQoS1 => 0x01);
define_reason_code!(GrantedQoS2 => 0x02);
define_reason_code!(DisconnectWithWillMessage => 0x04);
define_reason_code!(NoMatchingSubscribers => 0x10);
define_reason_code!(NoSubscriptionExisted => 0x11);
define_reason_code!(ContinueAuthentication => 0x18);
define_reason_code!(ReAuthenticate => 0x19);
define_reason_code!(UnspecifiedError => 0x80);
define_reason_code!(MalformedPacket => 0x81);
define_reason_code!(ProtocolError => 0x82);
define_reason_code!(ImplementationSpecificError => 0x83);
define_reason_code!(UnsupportedProtocolVersion => 0x84);
define_reason_code!(ClientIdentifierNotValid => 0x85);
define_reason_code!(BadUsernameOrPassword => 0x86);
define_reason_code!(NotAuthorized => 0x87);
define_reason_code!(ServerUnavailable => 0x88);
define_reason_code!(ServerBusy => 0x89);
define_reason_code!(Banned => 0x8A);
define_reason_code!(ServerShuttingDown => 0x8B);
define_reason_code!(BadAuthenticationMethod => 0x8C);
define_reason_code!(KeepAliveTimeout => 0x8D);
define_reason_code!(SessionTakenOver => 0x8E);
define_reason_code!(TopicFilterInvalid => 0x8F);
define_reason_code!(TopicNameInvalid => 0x90);
define_reason_code!(PacketIdentifierInUse => 0x91);
define_reason_code!(PacketIdentifierNotFound => 0x92);
define_reason_code!(ReceiveMaximumExceeded => 0x93);
define_reason_code!(TopicAliasInvalid => 0x94);
define_reason_code!(PacketTooLarge => 0x95);
define_reason_code!(MessageRateTooHigh => 0x96);
define_reason_code!(QuotaExceeded => 0x97);
define_reason_code!(AdministrativeAction => 0x98);
define_reason_code!(PayloadFormatInvalid => 0x99);
define_reason_code!(RetainNotSupported => 0x9A);
define_reason_code!(QoSNotSupported => 0x9B);
define_reason_code!(UseAnotherServer => 0x9C);
define_reason_code!(ServerMoved => 0x9D);
define_reason_code!(SharedSubscriptionsNotSupported => 0x9E);
define_reason_code!(ConnectionRateExceeded => 0x9F);
define_reason_code!(MaximumConnectTime => 0xA0);
define_reason_code!(SubscriptionIdentifiersNotSupported => 0xA1);
define_reason_code!(WildcardSubscriptionsNotSupported => 0xA2);
