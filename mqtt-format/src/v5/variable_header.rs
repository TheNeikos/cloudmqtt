use winnow::Bytes;

use super::{integers::parse_u16, integers::parse_u32, MResult};

pub struct PacketIdentifier(pub u16);

pub fn parse_packet_identifier<'i>(input: &mut &'i Bytes) -> MResult<PacketIdentifier> {
    parse_u16(input).map(PacketIdentifier)
}

macro_rules! define_properties {
    ($name:ident $(< $tylt:lifetime >)? as $id:expr => parse with $parser:path as $($lt:lifetime)? $kind:ty) => {
        define_properties!(@impl $name $($tylt)? as $id => $kind => |input: &mut &Bytes| -> MResult<Self> {
            Ok(Self($parser(input)?))
        });
    };

    ($name:ident $(< $tylt:lifetime >)? as $id:expr => parser: $parser:path as $($lt:lifetime)? $kind:ty) => {
        define_properties!(@impl $name $($tylt)? as $id => $kind => |input: &mut & $($tylt)? Bytes| -> MResult<Self> {
            #[allow(unused_imports)]
            use winnow::Parser;

            Ok(Self($parser.parse_next(input)?))
        });
    };

    (@impl $name:ident $($tylt:lifetime)? as $id:expr => $($lt:lifetime)? $kind:ty => $fun:expr) => {
        pub struct $name < $($tylt)? >(pub $(& $lt)? $kind);

        impl<$($tylt)?> $name < $($tylt)? > {
            const IDENTIFIER: u32 = $id;

            pub fn parse(input: &mut & $($tylt)? Bytes) -> MResult<$name <$($tylt)?>> {
                $fun(input)
            }
        }

        impl<'i> From< $name <$($tylt)?> > for Property<'i> {
            fn from(value: $name) -> Property<'i> {
                Property::$name(value)
            }
        }
    }
}

pub enum Property<'i> {
    PayloadFormatIndicator(PayloadFormatIndicator),
    MessageExpiryInterval(MessageExpiryInterval),
    ContentType(ContentType<'i>),
    ResponseTopic(ResponseTopic<'i>),
    CorrelationData(CorrelationData<'i>),
    SubscriptionIdentifier(SubscriptionIdentifier),
    SessionExpiryInterval(SessionExpiryInterval),
    AssignedClientIdentifier(AssignedClientIdentifier<'i>),
    ServerKeepAlive(ServerKeepAlive),
    AuthenticationMethod(AuthenticationMethod<'i>),
    AuthenticationData(AuthenticationData<'i>),
    RequestProblemInformation(RequestProblemInformation),
    WillDelayInterval(WillDelayInterval),
    RequestResponseInformation(RequestResponseInformation),
    ResponseInformation(ResponseInformation<'i>),
    ServerReference(ServerReference<'i>),
    ReasonString(ReasonString<'i>),
    ReceiveMaximum(ReceiveMaximum),
    TopicAliasMaximum(TopicAliasMaximum),
    TopicAlias(TopicAlias),
    MaximumQoS(MaximumQoS),
    RetainAvailable(RetainAvailable),
    UserProperty(UserProperty<'i>),
    MaximumPacketSize(MaximumPacketSize),
    WildcardSubscriptionAvailable(WildcardSubscriptionAvailable),
    SubscriptionIdentifiersAvailable(SubscriptionIdentifiersAvailable),
    SharedSubscriptionAvailable(SharedSubscriptionAvailable),
}

define_properties!(PayloadFormatIndicator as 0x01 => parse with winnow::binary::u8 as u8);
define_properties!(MessageExpiryInterval as 0x02 => parser: parse_u32 as u32);
define_properties!(ContentType<'i> as 0x03 => parser: super::strings::parse_string as &'i str);
define_properties!(ResponseTopic<'i> as 0x08 => parser: super::strings::parse_string as &'i str);
define_properties!(CorrelationData<'i> as 0x09 => parser: super::bytes::parse_data as &'i [u8]);
define_properties!(SubscriptionIdentifier as 0x0B => parser: parse_u32 as u32);
define_properties!(SessionExpiryInterval as 0x11 => parser: parse_u32 as u32);
define_properties!(AssignedClientIdentifier<'i> as 0x12 => parser: super::strings::parse_string as &'i str);
define_properties!(ServerKeepAlive as 0x13 => parser: parse_u32 as u32);
define_properties!(AuthenticationMethod<'i> as 0x15 => parser: super::strings::parse_string as &'i str);
define_properties!(AuthenticationData<'i> as 0x16 => parser: super::bytes::parse_data as &'i [u8]);
define_properties!(RequestProblemInformation as 0x17 => parse with winnow::binary::u8 as u8);
define_properties!(WillDelayInterval as 0x18 => parser: parse_u32 as u32);
define_properties!(RequestResponseInformation as 0x19 => parse with winnow::binary::u8 as u8);
define_properties!(ResponseInformation<'i> as 0x1A => parser: super::strings::parse_string as &'i str);
define_properties!(ServerReference<'i> as 0x1C => parser: super::strings::parse_string as &'i str);
define_properties!(ReasonString<'i> as 0x1F => parser: super::strings::parse_string as &'i str);
define_properties!(ReceiveMaximum as 0x21 => parser: parse_u32 as u32);
define_properties!(TopicAliasMaximum as 0x22 => parser: parse_u32 as u32);
define_properties!(TopicAlias as 0x23 => parser: parse_u32 as u32);
define_properties!(MaximumQoS as 0x24 => parse with winnow::binary::u8 as u8);
define_properties!(RetainAvailable as 0x25 => parse with winnow::binary::u8 as u8);
define_properties!(UserProperty<'i> as 0x26 => parser: super::strings::string_pair as (&'i str, &'i str ));
define_properties!(MaximumPacketSize as 0x27 => parser: parse_u32 as u32);
define_properties!(WildcardSubscriptionAvailable as 0x28 => parse with winnow::binary::u8 as u8);
define_properties!(SubscriptionIdentifiersAvailable as 0x29 => parse with winnow::binary::u8 as u8);
define_properties!(SharedSubscriptionAvailable as 0x2A => parse with winnow::binary::u8 as u8);
