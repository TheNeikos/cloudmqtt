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

                    Ok(Self(
                        winnow::combinator::trace(stringify!($name), $parser).parse_next(input)?
                    ))
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
            UserProperties(UserProperties<'i>),
        }

        impl<'i> Property<'i> {
            pub fn parse(input: &mut &'i Bytes) -> MResult<Property<'i>> {
                let disp = winnow::combinator::dispatch! { crate::v5::integers::parse_variable_u32;
                    $(
                        $id => $name::parse.map(Property::from),
                    )*
                    0x26 => UserProperties::parse.map(Property::from),
                    _ => winnow::combinator::fail
                };

                winnow::combinator::trace("Property", disp).parse_next(input)
            }
        }
    }
}

define_properties! {[
    PayloadFormatIndicator as 0x01 => parse with winnow::binary::u8 as u8,
    MessageExpiryInterval as 0x02 => parse with parse_u32 as u32,
    ContentType<'i> as 0x03 => parse with super::strings::parse_string as &'i str,
    ResponseTopic<'i> as 0x08 => parse with super::strings::parse_string as &'i str,
    CorrelationData<'i> as 0x09 => parse with super::bytes::parse_binary_data as &'i [u8],
    SubscriptionIdentifier as 0x0B => parse with parse_u32 as u32,
    SessionExpiryInterval as 0x11 => parse with parse_u32 as u32,
    AssignedClientIdentifier<'i> as 0x12 => parse with super::strings::parse_string as &'i str,
    ServerKeepAlive as 0x13 => parse with parse_u32 as u32,
    AuthenticationMethod<'i> as 0x15 => parse with super::strings::parse_string as &'i str,
    AuthenticationData<'i> as 0x16 => parse with super::bytes::parse_binary_data as &'i [u8],
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

pub struct UserProperties<'i>(pub &'i [u8]);

impl<'i> MqttProperties<'i> for UserProperties<'i> {
    const IDENTIFIER: u32 = 0x26;
    const ALLOW_REPEATING: bool = true;

    fn parse<'input>(input: &mut &'input Bytes) -> MResult<Self>
    where
        'input: 'i,
    {
        // We only need to verify there is a correct string pair
        let prop = crate::v5::strings::string_pair
            .recognize()
            .parse_next(input)?;

        Ok(Self(prop))
    }
}

impl<'i> From<UserProperties<'i>> for Property<'i> {
    fn from(value: UserProperties<'i>) -> Property<'i> {
        Property::UserProperties(value)
    }
}

impl<'i> UserProperties<'i> {
    pub fn iter(&'i self) -> UserPropertyIterator<'i> {
        // UserProperties (note the plural) points to the start of the first _valid_ UserProperty.
        // This means that the iterator first needs to consume that property before searching for
        // the next!
        UserPropertyIterator {
            current: Bytes::new(&self.0),
            first_prop: true,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct UserProperty<'i> {
    pub key: &'i str,
    pub value: &'i str,
}

impl<'i> UserProperty<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<UserProperty<'i>> {
        crate::v5::strings::string_pair
            .map(|(k, v)| UserProperty { key: k, value: v })
            .parse_next(input)
    }
}

pub struct UserPropertyIterator<'i> {
    current: &'i Bytes,
    first_prop: bool,
}

impl<'i> Iterator for UserPropertyIterator<'i> {
    type Item = UserProperty<'i>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_prop {
            self.first_prop = false;
            return Some(UserProperty::parse(&mut self.current).expect(
                "This has already been parsed and the first item should be a UserProperty",
            ));
        }

        while !self.current.is_empty() {
            let property = Property::parse(&mut self.current)
                .expect("This has already been parsed, and should be valid.");

            match property {
                Property::UserProperties(prop) => {
                    return Some(
                        UserProperty::parse(&mut Bytes::new(prop.0))
                            .expect("This has already been parsed and should be valid"),
                    )
                }
                _ => continue,
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::v5::variable_header::{MqttProperties, RetainAvailable, UserProperty};

    use super::UserProperties;

    #[test]
    fn check_iteration() {
        #[rustfmt::skip]
        let input = &[
            // First the string pair of the UserProp
            0x0, 0x1, b'a',
            0x0, 0x2, b'b', b'c',
            // Retain Available
            RetainAvailable::IDENTIFIER as u8,
            0x1,
            // User Property
            UserProperties::IDENTIFIER as u8,
            // Now a string pair
            0x0, 0x1, b'f',
            0x0, 0x2, b'h', b'j',
        ];

        let result = UserProperties(input);
        let props = result.iter().collect::<Vec<_>>();
        assert_eq!(props.len(), 2);
        assert_eq!(
            props[0],
            UserProperty {
                key: "a",
                value: "bc"
            }
        );
        assert_eq!(
            props[1],
            UserProperty {
                key: "f",
                value: "hj"
            }
        );
    }
}
