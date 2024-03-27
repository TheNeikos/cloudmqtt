//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::binary::bits::bits;
use winnow::combinator::repeat_till;
use winnow::error::ErrMode;
use winnow::error::InputError;
use winnow::error::ParserError;
use winnow::Bytes;
use winnow::Parser;

use crate::v5::fixed_header::QualityOfService;
use crate::v5::properties::define_properties;
use crate::v5::strings::parse_string;
use crate::v5::strings::write_string;
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::SubscriptionIdentifier;
use crate::v5::variable_header::UserProperties;
use crate::v5::write::WResult;
use crate::v5::write::WriteMqttPacket;
use crate::v5::MResult;

define_properties! {
    packet_type: MSubscribe,
    anker: "_Toc3901164",
    pub struct SubscribeProperties<'i> {
        (anker: "_Toc3901166")
        subscription_identifier: SubscriptionIdentifier,

        (anker: "_Toc3901167")
        user_properties: UserProperties<'i>,
    }
}

#[derive(Debug, num_enum::TryFromPrimitive, num_enum::IntoPrimitive, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum RetainHandling {
    SendRetainedMessagesAlways = 0,
    SendRetainedMessagesOnNewSubscribe = 1,
    DoNotSendRetainedMessages = 2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptionOptions {
    pub quality_of_service: QualityOfService,
    pub no_local: bool,
    pub retain_as_published: bool,
    pub retain_handling: RetainHandling,
}

impl SubscriptionOptions {
    fn parse(input: &mut &Bytes) -> MResult<SubscriptionOptions> {
        winnow::combinator::trace("SubscriptionOptions", |input: &mut &Bytes| {
            let (_reserved, retain_handling, retain_as_published, no_local, quality_of_service) =
                bits::<_, _, InputError<(_, usize)>, _, _>((
                    winnow::binary::bits::pattern(0x0, 2usize),
                    winnow::binary::bits::take(2usize)
                        .try_map(<RetainHandling as TryFrom<u8>>::try_from),
                    winnow::binary::bits::bool,
                    winnow::binary::bits::bool,
                    winnow::binary::bits::take(2usize)
                        .try_map(<QualityOfService as TryFrom<u8>>::try_from),
                ))
                .parse_next(input)
                .map_err(|_: ErrMode<InputError<_>>| {
                    ErrMode::from_error_kind(input, winnow::error::ErrorKind::Slice)
                })?;

            Ok(SubscriptionOptions {
                quality_of_service,
                no_local,
                retain_as_published,
                retain_handling,
            })
        })
        .parse_next(input)
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        let qos = self.quality_of_service as u8;
        let no_local = (self.no_local as u8) << 2;
        let retain_as_published = (self.retain_as_published as u8) << 3;
        let retain_handling = (self.retain_handling as u8) << 4;

        let sub_opts = qos | no_local | retain_as_published | retain_handling;

        buffer.write_byte(sub_opts)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901161")]
pub struct Subscription<'i> {
    pub topic_filter: &'i str,
    pub options: SubscriptionOptions,
}

impl<'i> Subscription<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Subscription<'i>> {
        winnow::combinator::trace("Subscription", |input: &mut &'i Bytes| {
            let (topic_filter, options) =
                (parse_string, SubscriptionOptions::parse).parse_next(input)?;

            Ok(Subscription {
                topic_filter,
                options,
            })
        })
        .parse_next(input)
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        write_string(buffer, self.topic_filter)?;
        self.options.write(buffer)
    }
}

#[derive(Clone)]
pub struct Subscriptions<'i> {
    start: &'i [u8],
}

impl<'i> core::cmp::PartialEq for Subscriptions<'i> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<'i> core::fmt::Debug for Subscriptions<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Subscriptions").finish()
    }
}

impl<'i> Subscriptions<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Subscriptions<'i>> {
        winnow::combinator::trace("Subscriptions", |input: &mut &'i Bytes| {
            let start = repeat_till::<_, _, (), _, _, _, _>(
                1..,
                Subscription::parse,
                winnow::combinator::eof,
            )
            .recognize()
            .parse_next(input)?;

            Ok(Subscriptions { start })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.start.len() as u32
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        for sub in self.iter() {
            sub.write(buffer)?;
        }

        Ok(())
    }

    pub fn iter(&self) -> SubscriptionsIter<'i> {
        SubscriptionsIter {
            current: Bytes::new(self.start),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct SubscriptionsIter<'i> {
    current: &'i Bytes,
}

impl<'i> Iterator for SubscriptionsIter<'i> {
    type Item = Subscription<'i>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.current.is_empty() {
            let sub = Subscription::parse(&mut self.current)
                .expect("Already parsed subscriptions should be valid");

            return Some(sub);
        }

        None
    }
}

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
pub struct MSubscribe<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: SubscribeProperties<'i>,
    pub subscriptions: Subscriptions<'i>,
}

impl<'i> MSubscribe<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MSubscribe<'i>> {
        winnow::combinator::trace("MSubscribe", |input: &mut &'i Bytes| {
            let (packet_identifier, properties) =
                (PacketIdentifier::parse, SubscribeProperties::parse).parse_next(input)?;

            let subscriptions = Subscriptions::parse(input)?;

            Ok(MSubscribe {
                packet_identifier,
                properties,
                subscriptions,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.packet_identifier.binary_size()
            + self.properties.binary_size()
            + self.subscriptions.binary_size()
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        self.packet_identifier.write(buffer)?;
        self.properties.write(buffer)?;
        self.subscriptions.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use crate::v5::fixed_header::QualityOfService;
    use crate::v5::packets::subscribe::MSubscribe;
    use crate::v5::packets::subscribe::RetainHandling;
    use crate::v5::packets::subscribe::SubscribeProperties;
    use crate::v5::packets::subscribe::Subscription;
    use crate::v5::packets::subscribe::SubscriptionOptions;
    use crate::v5::packets::subscribe::Subscriptions;
    use crate::v5::test::TestWriter;
    use crate::v5::variable_header::PacketIdentifier;
    use crate::v5::variable_header::SubscriptionIdentifier;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_subscription() {
        crate::v5::test::make_roundtrip_test!(Subscription {
            topic_filter: "foo",
            options: SubscriptionOptions {
                quality_of_service: QualityOfService::AtMostOnce,
                no_local: true,
                retain_as_published: true,
                retain_handling: RetainHandling::SendRetainedMessagesAlways,
            }
        });
    }

    #[test]
    fn test_roundtrip_subscribe_no_props() {
        let mut sub_writer = TestWriter { buffer: Vec::new() };

        let subscription = Subscription {
            topic_filter: "foo",
            options: SubscriptionOptions {
                quality_of_service: QualityOfService::AtMostOnce,
                no_local: true,
                retain_as_published: true,
                retain_handling: RetainHandling::SendRetainedMessagesAlways,
            },
        };

        subscription.write(&mut sub_writer).unwrap();

        crate::v5::test::make_roundtrip_test!(MSubscribe {
            packet_identifier: PacketIdentifier(88),
            subscriptions: Subscriptions {
                start: &sub_writer.buffer
            },
            properties: SubscribeProperties {
                subscription_identifier: None,
                user_properties: None,
            }
        });
    }

    #[test]
    fn test_roundtrip_subscribe_with_props() {
        let mut sub_writer = TestWriter { buffer: Vec::new() };

        let subscription = Subscription {
            topic_filter: "foo",
            options: SubscriptionOptions {
                quality_of_service: QualityOfService::AtMostOnce,
                no_local: true,
                retain_as_published: true,
                retain_handling: RetainHandling::SendRetainedMessagesAlways,
            },
        };

        subscription.write(&mut sub_writer).unwrap();

        crate::v5::test::make_roundtrip_test!(MSubscribe {
            packet_identifier: PacketIdentifier(88),
            subscriptions: Subscriptions {
                start: &sub_writer.buffer
            },
            properties: SubscribeProperties {
                subscription_identifier: Some(SubscriptionIdentifier(125)),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            }
        });
    }
}
