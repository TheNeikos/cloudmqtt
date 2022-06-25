use nom::{error::FromExternalError, multi::many1_count};

use super::{errors::MPacketHeaderError, MSResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MSubscriptionAcks<'message> {
    pub acks: &'message [MSubscriptionAck],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
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
    let acks = input;
    let (input, acks_len) = many1_count(msubscriptionack)(input)?;

    assert!(acks_len <= acks.len());

    let ack_ptr: *const MSubscriptionAck = acks.as_ptr() as *const MSubscriptionAck;
    let acks: &'message [MSubscriptionAck] = unsafe {
        // SAFETY: The array has been checked and is of the correct len, as well as
        // MSubscriptionAck is the same repr and has no padding
        std::slice::from_raw_parts(ack_ptr, acks_len)
    };

    Ok((input, MSubscriptionAcks { acks }))
}

#[cfg(test)]
mod tests {
    use crate::v3::subscription_acks::MSubscriptionAck;

    use super::msubscriptionacks;

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
