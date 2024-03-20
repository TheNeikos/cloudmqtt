use winnow::{Bytes, Parser};

use super::{integers::parse_u16, integers::parse_u32, MResult};

pub struct PacketIdentifier(pub u16);

pub fn parse_packet_identifier<'i>(input: &mut &'i Bytes) -> MResult<PacketIdentifier> {
    parse_u16(input).map(PacketIdentifier)
}

pub trait MqttProperties<'lt>: Sized {
    const IDENTIFIER: u32;
    const ALLOW_REPEATING: bool;

    fn parse<'input>(input: &mut &'input Bytes) -> MResult<Self>
    where
        'input: 'lt;
}

macro_rules! define_properties {
    ([
        $(
            $name:ident $(< $tylt:lifetime >)? as $id:expr => parse with $parser:path as $($lt:lifetime)? $kind:ty
        ),*
        $(,)?
    ]) => {
        $(
            pub struct $name < $($tylt)? >(pub $(& $lt)? $kind);

            impl<'lt $(, $tylt)?> MqttProperties<'lt> for $name < $($tylt)? >
                $(where $tylt: 'lt, 'lt: $tylt)?
            {
                const IDENTIFIER: u32 = $id;
                const ALLOW_REPEATING: bool = false;

                fn parse<'input>(input: &mut &'input Bytes) -> MResult<$name <$($tylt)?>>
                    where
                        'input: 'lt
                {
                    use winnow::Parser;

                    Ok(Self($parser.parse_next(input)?))
                }
            }

            impl<'i> From< $name <$($tylt)?> > for Property<'i> {
                fn from(value: $name <$($tylt)?>) -> Property<'i> {
                    Property::$name(value)
                }
            }
        )*

        pub enum Property<'i> {
            $(
                $name ( $name $(< $tylt >)? ),
            )*
            UserProperty(UserProperty<'i>),
        }

        impl<'i> Property<'i> {
            pub fn parse(input: &mut &'i Bytes) -> MResult<Property<'i>> {
                winnow::combinator::dispatch! { crate::v5::integers::parse_variable_u32;
                    $(
                        $id => $name::parse.map(Property::from),
                    )*
                    _ => winnow::combinator::fail
                }.parse_next(input)
            }
        }
    }
}

define_properties! {[
    PayloadFormatIndicator as 0x01 => parse with winnow::binary::u8 as u8,
    MessageExpiryInterval as 0x02 => parse with parse_u32 as u32,
    ContentType<'i> as 0x03 => parse with super::strings::parse_string as &'i str,
    ResponseTopic<'i> as 0x08 => parse with super::strings::parse_string as &'i str,
    CorrelationData<'i> as 0x09 => parse with super::bytes::parse_data as &'i [u8],
    SubscriptionIdentifier as 0x0B => parse with parse_u32 as u32,
    SessionExpiryInterval as 0x11 => parse with parse_u32 as u32,
    AssignedClientIdentifier<'i> as 0x12 => parse with super::strings::parse_string as &'i str,
    ServerKeepAlive as 0x13 => parse with parse_u32 as u32,
    AuthenticationMethod<'i> as 0x15 => parse with super::strings::parse_string as &'i str,
    AuthenticationData<'i> as 0x16 => parse with super::bytes::parse_data as &'i [u8],
    RequestProblemInformation as 0x17 => parse with winnow::binary::u8 as u8,
    WillDelayInterval as 0x18 => parse with parse_u32 as u32,
    RequestResponseInformation as 0x19 => parse with winnow::binary::u8 as u8,
    ResponseInformation<'i> as 0x1A => parse with super::strings::parse_string as &'i str,
    ServerReference<'i> as 0x1C => parse with super::strings::parse_string as &'i str,
    ReasonString<'i> as 0x1F => parse with super::strings::parse_string as &'i str,
    ReceiveMaximum as 0x21 => parse with parse_u32 as u32,
    TopicAliasMaximum as 0x22 => parse with parse_u32 as u32,
    TopicAlias as 0x23 => parse with parse_u32 as u32,
    MaximumQoS as 0x24 => parse with winnow::binary::u8 as u8,
    RetainAvailable as 0x25 => parse with winnow::binary::u8 as u8,
    MaximumPacketSize as 0x27 => parse with parse_u32 as u32,
    WildcardSubscriptionAvailable as 0x28 => parse with winnow::binary::u8 as u8,
    SubscriptionIdentifiersAvailable as 0x29 => parse with winnow::binary::u8 as u8,
    SharedSubscriptionAvailable as 0x2A => parse with winnow::binary::u8 as u8,
]}

pub struct UserProperty<'i>(pub &'i [u8]);

impl<'i> MqttProperties<'i> for UserProperty<'i> {
    const IDENTIFIER: u32 = 0x26;
    const ALLOW_REPEATING: bool = true;

    fn parse<'input>(input: &mut &'input Bytes) -> MResult<Self>
    where
        'input: 'i,
    {
        let checkpoint = *input;

        // We only need to verify there is a correct string pair
        let _ = crate::v5::strings::string_pair.parse_peek(input)?;

        Ok(Self(checkpoint.as_ref()))
    }
}

impl<'i> From<UserProperty<'i>> for Property<'i> {
    fn from(value: UserProperty<'i>) -> Property<'i> {
        Property::UserProperty(value)
    }
}
