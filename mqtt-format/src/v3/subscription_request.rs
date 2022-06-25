use nom::{multi::many1_count, Parser};
use nom_supreme::ParserExt;

use super::{
    qos::{mquality_of_service, MQualityOfService},
    strings::{mstring, MString},
    MSResult,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MSubscriptionRequests<'message> {
    count: usize,
    data: &'message [u8],
}

impl<'message> IntoIterator for MSubscriptionRequests<'message> {
    type Item = MSubscriptionRequest<'message>;

    type IntoIter = MSubscriptionIter<'message>;

    fn into_iter(self) -> Self::IntoIter {
        MSubscriptionIter {
            count: self.count,
            data: self.data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MSubscriptionIter<'message> {
    count: usize,
    data: &'message [u8],
}

impl<'message> Iterator for MSubscriptionIter<'message> {
    type Item = MSubscriptionRequest<'message>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let (rest, request) =
            msubscriptionrequest(self.data).expect("Could not parse already validated sub request");
        self.data = rest;

        Some(request)
    }
}

pub fn msubscriptionrequests(input: &[u8]) -> MSResult<'_, MSubscriptionRequests<'_>> {
    let data = input;
    let (input, count) = many1_count(msubscriptionrequest)(input)?;

    Ok((input, MSubscriptionRequests { count, data }))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MSubscriptionRequest<'message> {
    pub topic: MString<'message>,
    pub qos: MQualityOfService,
}

pub fn msubscriptionrequest(input: &[u8]) -> MSResult<'_, MSubscriptionRequest<'_>> {
    let (input, topic) = mstring(input)?;
    let (input, qos) = nom::number::complete::u8
        .map_res(mquality_of_service)
        .parse(input)?;

    Ok((input, MSubscriptionRequest { topic, qos }))
}

#[cfg(test)]
mod tests {
    use crate::v3::{strings::MString, subscription_request::MSubscriptionRequest};

    use super::msubscriptionrequests;

    #[test]
    fn test_subscription_iterator() {
        let input = &[
            0, 3, // Length 3
            0x61, 0x2F, 0x62, // The string 'a/b'
            1,    // QoS 1
            0, 3, // Length 3
            0x63, 0x2F, 0x64, // The string 'c/d'
            2,    // QoS 2
        ];

        let (rest, subs) = msubscriptionrequests(input).unwrap();

        assert_eq!(rest, &[]);

        let mut sub_iter = subs.into_iter();

        assert_eq!(
            sub_iter.next(),
            Some(MSubscriptionRequest {
                qos: crate::v3::qos::MQualityOfService::AtLeastOnce,
                topic: MString { value: "a/b" },
            })
        );

        assert_eq!(
            sub_iter.next(),
            Some(MSubscriptionRequest {
                qos: crate::v3::qos::MQualityOfService::ExactlyOnce,
                topic: MString { value: "c/d" },
            })
        );

        assert_eq!(sub_iter.next(), None,);
    }
}
