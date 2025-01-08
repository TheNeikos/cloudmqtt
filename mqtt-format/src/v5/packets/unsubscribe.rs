//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::combinator::repeat_till;
use winnow::Bytes;
use winnow::Parser;

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
    packet_type: MUnsubscribe,
    anker: "_Toc3901182",
    pub struct UnsubscribeProperties<'i> {
        (anker: "_Toc3901183")
        subscription_identifier: SubscriptionIdentifier,

        (anker: "_Toc3901183")
        user_properties: UserProperties<'i>,
    }
}

#[derive(Clone)]
pub struct Unsubscriptions<'i> {
    start: &'i [u8],
}

impl<'i> core::cmp::PartialEq for Unsubscriptions<'i> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<'i> core::fmt::Debug for Unsubscriptions<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Unsubscriptions").finish()
    }
}

impl<'i> Unsubscriptions<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Unsubscriptions<'i>> {
        winnow::combinator::trace("Unsubscriptions", |input: &mut &'i Bytes| {
            let start = repeat_till::<_, _, (), _, _, _, _>(
                1..,
                Unsubscription::parse,
                winnow::combinator::eof,
            )
            .take()
            .parse_next(input)?;

            Ok(Unsubscriptions { start })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.start.len() as u32
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        for unsub in self.iter() {
            unsub.write(buffer)?;
        }

        Ok(())
    }

    pub fn iter(&self) -> UnsubscriptionsIter<'i> {
        UnsubscriptionsIter {
            current: Bytes::new(self.start),
        }
    }
}

#[allow(missing_debug_implementations)]
pub struct UnsubscriptionsIter<'i> {
    current: &'i Bytes,
}

impl<'i> Iterator for UnsubscriptionsIter<'i> {
    type Item = Unsubscription<'i>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.current.is_empty() {
            let sub = Unsubscription::parse(&mut self.current)
                .expect("Already parsed subscriptions should be valid");

            return Some(sub);
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unsubscription<'i> {
    pub topic_filter: &'i str,
}

impl<'i> Unsubscription<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("Unsubscription", |input: &mut &'i Bytes| {
            let topic_filter = parse_string(input)?;

            Ok(Unsubscription { topic_filter })
        })
        .parse_next(input)
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        write_string(buffer, self.topic_filter)
    }
}

#[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
#[derive(Clone, Debug, PartialEq)]
#[doc = crate::v5::util::md_speclink!("_Toc3901179")]
pub struct MUnsubscribe<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: UnsubscribeProperties<'i>,
    pub unsubscriptions: Unsubscriptions<'i>,
}

impl<'i> MUnsubscribe<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        winnow::combinator::trace("MUnsubscribe", |input: &mut &'i Bytes| {
            let (packet_identifier, properties, unsubscriptions) = (
                PacketIdentifier::parse,
                UnsubscribeProperties::parse,
                Unsubscriptions::parse,
            )
                .parse_next(input)?;

            Ok(MUnsubscribe {
                packet_identifier,
                properties,
                unsubscriptions,
            })
        })
        .parse_next(input)
    }

    pub fn binary_size(&self) -> u32 {
        self.packet_identifier.binary_size()
            + self.properties.binary_size()
            + self.unsubscriptions.binary_size()
    }

    pub fn write<W: WriteMqttPacket>(&self, buffer: &mut W) -> WResult<W> {
        self.packet_identifier.write(buffer)?;
        self.properties.write(buffer)?;
        self.unsubscriptions.write(buffer)
    }
}

#[cfg(test)]
mod test {
    use crate::v5::packets::unsubscribe::MUnsubscribe;
    use crate::v5::packets::unsubscribe::UnsubscribeProperties;
    use crate::v5::packets::unsubscribe::Unsubscription;
    use crate::v5::packets::unsubscribe::Unsubscriptions;
    use crate::v5::test::TestWriter;
    use crate::v5::variable_header::PacketIdentifier;
    use crate::v5::variable_header::SubscriptionIdentifier;
    use crate::v5::variable_header::UserProperties;

    #[test]
    fn test_roundtrip_unsubscription() {
        crate::v5::test::make_roundtrip_test!(Unsubscription {
            topic_filter: "foo",
        });
    }

    #[test]
    fn test_roundtrip_unsubscribe_no_props() {
        let mut sub_writer = TestWriter { buffer: Vec::new() };

        let subscription = Unsubscription {
            topic_filter: "foo",
        };

        subscription.write(&mut sub_writer).unwrap();

        crate::v5::test::make_roundtrip_test!(MUnsubscribe {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(88).unwrap()),
            unsubscriptions: Unsubscriptions {
                start: &sub_writer.buffer
            },
            properties: UnsubscribeProperties {
                subscription_identifier: None,
                user_properties: None,
            }
        });
    }

    #[test]
    fn test_roundtrip_unsubscribe_with_props() {
        let mut sub_writer = TestWriter { buffer: Vec::new() };

        let subscription = Unsubscription {
            topic_filter: "foo",
        };

        subscription.write(&mut sub_writer).unwrap();

        crate::v5::test::make_roundtrip_test!(MUnsubscribe {
            packet_identifier: PacketIdentifier(core::num::NonZeroU16::new(88).unwrap()),
            unsubscriptions: Unsubscriptions {
                start: &sub_writer.buffer
            },
            properties: UnsubscribeProperties {
                subscription_identifier: Some(SubscriptionIdentifier(125)),
                user_properties: Some(UserProperties(&[0x0, 0x1, b'f', 0x0, 0x2, b'h', b'j'])),
            }
        });
    }
}
