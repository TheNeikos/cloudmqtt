use winnow::{combinator::repeat_till, Bytes, Parser};

use crate::v5::{
    properties::define_properties,
    strings::parse_string,
    variable_header::{PacketIdentifier, SubscriptionIdentifier, UserProperties},
    MResult,
};

define_properties! {
    pub struct UnsubscribeProperties<'i> {
        subscription_identifier: SubscriptionIdentifier,
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

pub struct MUnsubscribe<'i> {
    packet_identifier: PacketIdentifier,
    properties: UnsubscribeProperties<'i>,
    unsubscriptions: Unsubscriptions<'i>,
}

impl<'i> MUnsubscribe<'i> {
    fn parse(input: &mut &'i Bytes) -> MResult<Self> {
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
