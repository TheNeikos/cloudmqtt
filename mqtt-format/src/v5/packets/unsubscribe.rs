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
use crate::v5::variable_header::PacketIdentifier;
use crate::v5::variable_header::SubscriptionIdentifier;
use crate::v5::variable_header::UserProperties;
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

pub struct Unsubscriptions<'i> {
    start: &'i [u8],
}

impl<'i> std::fmt::Debug for Unsubscriptions<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            .recognize()
            .parse_next(input)?;

            Ok(Unsubscriptions { start })
        })
        .parse_next(input)
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

#[derive(Debug)]
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
}

#[derive(Debug)]
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
}
