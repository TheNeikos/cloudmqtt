//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use bytemuck::CheckedBitPattern;
use bytemuck::NoUninit;
use futures::AsyncWrite;
use futures::AsyncWriteExt;
use nom::combinator::recognize;
use nom::error::FromExternalError;
use nom::multi::many1_count;

use super::errors::MPacketHeaderError;
use super::errors::MPacketWriteError;
use super::MSResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MSubscriptionAcks<'message> {
    pub acks: &'message [MSubscriptionAck],
}
impl MSubscriptionAcks<'_> {
    pub(crate) async fn write_to<W: AsyncWrite>(
        &self,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer.write_all(bytemuck::cast_slice(self.acks)).await?;
        Ok(())
    }
    pub(crate) fn get_len(&self) -> usize {
        1
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, CheckedBitPattern, NoUninit)]
pub enum MSubscriptionAck {
    MaximumQualityAtMostOnce = 0x00,
    MaximumQualityAtLeastOnce = 0x01,
    MaximumQualityExactlyOnce = 0x02,
    Failure = 0x80,
}

fn msubscriptionack(input: &[u8]) -> MSResult<'_, MSubscriptionAck> {
    let (input, data) = nom::number::complete::u8(input)?;

    Ok((
        input,
        match data {
            0x00 => MSubscriptionAck::MaximumQualityAtMostOnce,
            0x01 => MSubscriptionAck::MaximumQualityAtLeastOnce,
            0x02 => MSubscriptionAck::MaximumQualityExactlyOnce,
            0x80 => MSubscriptionAck::Failure,
            invalid_ack => {
                return Err(nom::Err::Error(nom::error::Error::from_external_error(
                    input,
                    nom::error::ErrorKind::MapRes,
                    MPacketHeaderError::InvalidSubscriptionAck(invalid_ack),
                )))
            }
        },
    ))
}

pub fn msubscriptionacks<'message>(
    input: &'message [u8],
) -> MSResult<'message, MSubscriptionAcks<'message>> {
    let (input, acks) = recognize(many1_count(msubscriptionack))(input)?;

    let acks: &'message [MSubscriptionAck] = bytemuck::checked::cast_slice(acks);

    Ok((input, MSubscriptionAcks { acks }))
}

#[cfg(test)]
mod tests {
    use super::msubscriptionacks;
    use crate::v3::subscription_acks::MSubscriptionAck;

    #[test]
    fn check_valid_subacks() {
        let input = &[0x1, 0x2, 0x0, 0x80];

        let (rest, sub_acks) = msubscriptionacks(input).unwrap();

        assert_eq!(rest, &[]);
        assert_eq!(
            sub_acks.acks,
            &[
                MSubscriptionAck::MaximumQualityAtLeastOnce,
                MSubscriptionAck::MaximumQualityExactlyOnce,
                MSubscriptionAck::MaximumQualityAtMostOnce,
                MSubscriptionAck::Failure,
            ]
        )
    }

    #[test]
    fn check_invalid_subacks() {
        let input = &[0x1, 0x5];

        nom::combinator::all_consuming(msubscriptionacks)(input).unwrap_err();
    }
}
