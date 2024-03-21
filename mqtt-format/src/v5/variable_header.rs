//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Various components present in MQTT variable headers

use winnow::Bytes;
use winnow::Parser;

use super::integers::parse_u16;
use super::integers::parse_u32;
use super::write::WResult;
use super::write::WriteMqttPacket;
use super::MResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PacketIdentifier(pub u16);

impl PacketIdentifier {
    pub fn parse(input: &mut &Bytes) -> MResult<Self> {
        winnow::combinator::trace("PacketIdentifier", |input: &mut &Bytes| {
            parse_u16(input).map(PacketIdentifier)
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        2
    }

    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        buffer.write_u16(self.0).await
    }
}

pub trait MqttProperties<'lt>: Sized {
    const IDENTIFIER: u32;
    const ALLOW_REPEATING: bool;

    fn parse<'input>(input: &mut &'input Bytes) -> MResult<Self>
    where
        'input: 'lt;

    fn binary_size(&self) -> u32;

    fn write<W: WriteMqttPacket>(
        &self,
        buffer: &mut W,
    ) -> impl core::future::Future<Output = WResult<W>> + Send;
}

macro_rules! define_properties {
    ([
        $(
            $name:ident $(< $tylt:lifetime >)? as $id:expr =>
                parse with $parser:path as $($lt:lifetime)? $kind:ty;
                write with $writer:path;
                with size $size_closure:expr
        ),*
        $(,)?
    ]) => {
        $(
            #[derive(Debug, PartialEq)]
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
                    winnow::combinator::trace("PacketIdentifier", |input: &mut &'input Bytes| {
                        Ok(Self(
                            winnow::combinator::trace(stringify!($name), $parser).parse_next(input)?
                        ))
                    }).parse_next(input)
                }

                fn binary_size(&self) -> u32 {
                    let fun = $size_closure;
                    fun(&self.0)
                }

                async fn write<W: $crate::v5::write::WriteMqttPacket>(&self, buffer: &mut W)
                    -> $crate::v5::write::WResult<W>
                {
                    $crate::v5::integers::write_variable_u32(buffer, $id).await?;
                    $writer(buffer, self.0).await?;
                    Ok(())
                }
            }

            impl<'i> From< $name <$($tylt)?> > for Property<'i> {
                fn from(value: $name <$($tylt)?>) -> Property<'i> {
                    Property::$name(value)
                }
            }
        )*

        #[derive(Debug, PartialEq)]
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
                    _ => winnow::combinator::trace("Unknown property id", winnow::combinator::fail),
                };

                winnow::combinator::trace("Property", disp).parse_next(input)
            }

            pub async fn write<W: $crate::v5::write::WriteMqttPacket>(&self, buffer: &mut W)
                -> $crate::v5::write::WResult<W>
            {
                match self {
                    $(
                        Self::$name (inner) => inner.write(buffer).await?,
                    )*

                    Self::UserProperties(inner) => inner.write(buffer).await?,
                }

                Ok(())
            }
        }
    }
}

#[inline]
async fn write_u8<W: WriteMqttPacket>(buffer: &mut W, u: u8) -> WResult<W> {
    buffer.write_byte(u).await
}

define_properties! {[
    PayloadFormatIndicator as 0x01 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    MessageExpiryInterval as 0x02 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    ContentType<'i> as 0x03 =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    ResponseTopic<'i> as 0x08 =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    CorrelationData<'i> as 0x09 =>
        parse with super::bytes::parse_binary_data as &'i [u8];
        write with super::bytes::write_binary_data;
        with size super::bytes::binary_data_binary_size,

    SubscriptionIdentifier as 0x0B =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    SessionExpiryInterval as 0x11 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    AssignedClientIdentifier<'i> as 0x12 =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    ServerKeepAlive as 0x13 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    AuthenticationMethod<'i> as 0x15 =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    AuthenticationData<'i> as 0x16 =>
        parse with super::bytes::parse_binary_data as &'i [u8];
        write with super::bytes::write_binary_data;
        with size super::bytes::binary_data_binary_size,

    RequestProblemInformation as 0x17 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    WillDelayInterval as 0x18 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    RequestResponseInformation as 0x19 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    ResponseInformation<'i> as 0x1A =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    ServerReference<'i> as 0x1C =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    ReasonString<'i> as 0x1F =>
        parse with super::strings::parse_string as &'i str;
        write with super::strings::write_string;
        with size super::strings::string_binary_size,

    ReceiveMaximum as 0x21 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    TopicAliasMaximum as 0x22 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    TopicAlias as 0x23 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    MaximumQoS as 0x24 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    RetainAvailable as 0x25 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    MaximumPacketSize as 0x27 =>
        parse with parse_u32 as u32;
        write with super::integers::write_u32;
        with size |_| 4,

    WildcardSubscriptionAvailable as 0x28 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    SubscriptionIdentifiersAvailable as 0x29 =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,

    SharedSubscriptionAvailable as 0x2A =>
        parse with winnow::binary::u8 as u8;
        write with write_u8;
        with size |_| 1,
]}

pub struct UserProperties<'i>(pub &'i [u8]);

impl<'i> core::cmp::PartialEq for UserProperties<'i> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<'i> core::fmt::Debug for UserProperties<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("UserProperties").finish()
    }
}

impl<'i> MqttProperties<'i> for UserProperties<'i> {
    const IDENTIFIER: u32 = 0x26;
    const ALLOW_REPEATING: bool = true;

    fn parse<'input>(input: &mut &'input Bytes) -> MResult<Self>
    where
        'input: 'i,
    {
        winnow::combinator::trace("UserProperties", |input: &mut &'input Bytes| {
            let slice = *input;

            // We only need to verify there is a correct string pair
            let _prop = UserProperty::parse.recognize().parse_next(input)?;

            Ok(Self(slice))
        })
        .parse_next(input)
    }

    fn binary_size(&self) -> u32 {
        self.iter()
            .map(|up| {
                crate::v5::integers::variable_u32_binary_size(Self::IDENTIFIER) + up.binary_size()
            })
            .sum()
    }

    async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        for up in self.iter() {
            crate::v5::integers::write_variable_u32(buffer, Self::IDENTIFIER).await?;
            up.write(buffer).await?;
        }

        Ok(())
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
        winnow::combinator::trace("UserProperty", |input: &mut &'i Bytes| {
            crate::v5::strings::parse_string_pair
                .map(|(k, v)| UserProperty { key: k, value: v })
                .parse_next(input)
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        crate::v5::strings::string_pair_binary_size(self.key, self.value)
    }

    pub async fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        crate::v5::strings::write_string(buffer, self.key).await?;
        crate::v5::strings::write_string(buffer, self.value).await
    }
}

#[allow(missing_debug_implementations)]
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
    use winnow::Bytes;

    use super::UserProperties;
    use crate::v5::test::TestWriter;
    use crate::v5::variable_header::MqttProperties;
    use crate::v5::variable_header::Property;
    use crate::v5::variable_header::RetainAvailable;
    use crate::v5::variable_header::UserProperty;

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

    #[tokio::test]
    async fn test_write_properties() {
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

        let prop = UserProperties(input);

        let mut writer = TestWriter { buffer: Vec::new() };
        Property::UserProperties(prop)
            .write(&mut writer)
            .await
            .unwrap();

        let out = Property::parse(&mut Bytes::new(&writer.buffer)).unwrap();

        match out {
            Property::UserProperties(up) => {
                assert_eq!(
                    up.iter().collect::<Vec<_>>(),
                    UserProperties(input).iter().collect::<Vec<_>>()
                );
            }

            _ => panic!("Wrong type"),
        }
    }
}
