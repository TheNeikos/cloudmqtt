//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use futures::AsyncWrite;
use futures::AsyncWriteExt;
use nom::multi::many1_count;
use nom::Parser;
use nom_supreme::ParserExt;

use super::errors::MPacketWriteError;
use super::qos::mquality_of_service;
use super::qos::MQualityOfService;
use super::strings::mstring;
use super::strings::MString;
use super::MSResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MSubscriptionRequests<'message> {
    pub count: usize,
    pub data: &'message [u8],
}

impl MSubscriptionRequests<'_> {
    pub(crate) async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer.write_all(self.data).await?;
        Ok(())
    }
    pub(crate) fn get_len(&self) -> usize {
        self.data.len()
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        match msubscriptionrequest(self.data) {
            Ok((rest, request)) => {
                self.data = rest;
                Some(request)
            }
            Err(e) => {
                unreachable!("Could not parse already validated sub request: {}", e)
            }
        }
    }
}

pub fn msubscriptionrequests(input: &[u8]) -> MSResult<'_, MSubscriptionRequests<'_>> {
    let data = input;
    let (input, count) = many1_count(msubscriptionrequest)(input)?;

    Ok((input, MSubscriptionRequests { count, data }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MSubscriptionRequest<'message> {
    pub topic: MString<'message>,
    pub qos: MQualityOfService,
}

impl MSubscriptionRequest<'_> {
    pub async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        MString::write_to(&self.topic, writer).await?;
        self.qos.write_to(writer).await?;

        Ok(())
    }
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
    use super::msubscriptionrequests;
    use crate::v3::strings::MString;
    use crate::v3::subscription_request::MSubscriptionRequest;

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
