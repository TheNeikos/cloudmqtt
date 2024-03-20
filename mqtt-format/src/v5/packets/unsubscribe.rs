use winnow::{combinator::repeat_till, Bytes, Parser};

use crate::v5::{
    properties::define_properties,
    strings::parse_string,
    variable_header::{PacketIdentifier, SubscriptionIdentifier, UserProperties},
    MResult,
};

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

pub struct Unsubscriptions<'i> {
    start: &'i [u8],
}

impl<'i> Unsubscriptions<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Unsubscriptions<'i>> {
        let start = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            Unsubscription::parse,
            winnow::combinator::eof,
        )
        .recognize()
        .parse_next(input)?;

        Ok(Unsubscriptions { start })
    }

    pub fn iter(&self) -> UnsubscriptionsIter<'i> {
        UnsubscriptionsIter {
            current: Bytes::new(self.start),
        }
    }
}

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

pub struct Unsubscription<'i> {
    pub topic_filter: &'i str,
}

impl<'i> Unsubscription<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Self> {
        let topic_filter = parse_string(input)?;

        Ok(Unsubscription { topic_filter })
    }
}

#[doc = crate::v5::util::md_speclink!("_Toc3901179")]
pub struct MUnsubscribe<'i> {
    pub packet_identifier: PacketIdentifier,
    pub properties: UnsubscribeProperties<'i>,
    pub unsubscriptions: Unsubscriptions<'i>,
}

impl<'i> MUnsubscribe<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<Self> {
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
    }
}
