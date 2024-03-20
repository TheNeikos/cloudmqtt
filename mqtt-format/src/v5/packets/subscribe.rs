use winnow::{
    binary::bits::bits,
    combinator::repeat_till,
    error::{ErrMode, InputError, ParserError},
    Bytes, Parser,
};

use crate::v5::{
    fixed_header::QualityOfService,
    properties::define_properties,
    strings::parse_string,
    variable_header::{PacketIdentifier, SubscriptionIdentifier, UserProperties},
    MResult,
};

define_properties! {
    pub struct SubscribeProperties<'i> {
        subscription_identifier: SubscriptionIdentifier,
        user_properties: UserProperties<'i>,
    }
}

#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum RetainHandling {
    SendRetainedMessagesAlways = 0,
    SendRetainedMessagesOnNewSubscribe = 1,
    DoNotSendRetainedMessages = 2,
}

pub struct SubscriptionOptions {
    pub quality_of_service: QualityOfService,
    pub no_local: bool,
    pub retain_as_published: bool,
    pub retain_handling: RetainHandling,
}

impl SubscriptionOptions {
    fn parse(input: &mut &Bytes) -> MResult<SubscriptionOptions> {
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
    }
}

pub struct Subscription<'i> {
    pub topic_filter: &'i str,
    pub options: SubscriptionOptions,
}

impl<'i> Subscription<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Subscription<'i>> {
        let (topic_filter, options) =
            (parse_string, SubscriptionOptions::parse).parse_next(input)?;

        Ok(Subscription {
            topic_filter,
            options,
        })
    }
}

pub struct Subscriptions<'i> {
    start: &'i [u8],
}

impl<'i> Subscriptions<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Subscriptions<'i>> {
        let start =
            repeat_till::<_, _, (), _, _, _, _>(1.., Subscription::parse, winnow::combinator::eof)
                .recognize()
                .parse_next(input)?;

        Ok(Subscriptions { start })
    }

    pub fn iter(&self) -> SubscriptionsIter<'i> {
        SubscriptionsIter {
            current: Bytes::new(self.start),
        }
    }
}

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

pub struct MSubscribe<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: SubscribeProperties<'i>,
    pub subscriptions: Subscriptions<'i>,
}

impl<'i> MSubscribe<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MSubscribe<'i>> {
        let (packet_identifier, properties) =
            (PacketIdentifier::parse, SubscribeProperties::parse).parse_next(input)?;

        let subscriptions = Subscriptions::parse(input)?;

        Ok(MSubscribe {
            packet_identifier,
            properties,
            subscriptions,
        })
    }
}
