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
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::SubscriptionIdentifier;
use crate::v5::variable_header::UserProperties;
use crate::v5::MResult;

define_properties! {
    pub struct SubscribeProperties<'i> {
        subscription_identifier: SubscriptionIdentifier,
        user_properties: UserProperties<'i>,
    }
}

#[derive(Debug, num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum RetainHandling {
    SendRetainedMessagesAlways = 0,
    SendRetainedMessagesOnNewSubscribe = 1,
    DoNotSendRetainedMessages = 2,
}

#[derive(Debug)]
pub struct SubscriptionOptions {
    pub quality_of_service: QualityOfService,
    pub no_local: bool,
    pub retain_as_published: bool,
    pub retain_handling: RetainHandling,
}

impl SubscriptionOptions {
    fn parse(input: &mut &Bytes) -> MResult<SubscriptionOptions> {
        winnow::combinator::trace("SubscriptionOptions", |input: &mut &Bytes| {
            let (quality_of_service, no_local, retain_as_published, retain_handling) =
                bits::<_, _, InputError<(_, usize)>, _, _>((
                    winnow::binary::bits::take(2usize)
                        .try_map(<QualityOfService as TryFrom<u8>>::try_from),
                    winnow::binary::bits::bool,
                    winnow::binary::bits::bool,
                    winnow::binary::bits::take(2usize)
                        .try_map(<RetainHandling as TryFrom<u8>>::try_from),
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
}

#[derive(Debug)]
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
}

pub struct Subscriptions<'i> {
    start: &'i [u8],
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

#[derive(Debug)]
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
}
