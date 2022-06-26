use futures::{AsyncWrite, AsyncWriteExt};
use nom::{bytes::complete::take, number::complete::be_u16, IResult, Parser};
use nom_supreme::ParserExt;

use super::errors::MPacketWriteError;

/// A v3 MQTT string as defined in section 1.5.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MString<'message> {
    pub value: &'message str,
}

impl<'message> std::ops::Deref for MString<'message> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'message> MString<'message> {
    pub fn get_len(mstr: &MString<'_>) -> usize {
        2 + mstr.value.len()
    }

    pub(crate) async fn write_to<W: AsyncWrite>(
        mstr: &MString<'_>,
        writer: &mut std::pin::Pin<&mut W>,
    ) -> Result<(), MPacketWriteError> {
        writer
            .write_all(&(mstr.value.len() as u16).to_be_bytes())
            .await?;
        writer.write_all(mstr.value.as_bytes()).await?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MStringError {
    #[error("The input contained control characters, which this implementation rejects.")]
    ControlCharacters,
}

fn control_characters(c: char) -> bool {
    ('\u{0001}'..='\u{001F}').contains(&c) || ('\u{007F}'..='\u{009F}').contains(&c)
}

pub fn mstring(input: &[u8]) -> IResult<&[u8], MString<'_>> {
    let len = be_u16;
    let string_data = len.flat_map(take);

    string_data
        .map_res(|data| std::str::from_utf8(data).map(|s| MString { value: s }))
        .map_res(|s| {
            if s.contains(control_characters) {
                Err(MStringError::ControlCharacters)
            } else {
                Ok(s)
            }
        })
        .parse(input)
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;

    use super::{mstring, MString};

    // TODO(neikos): Unclear how MQTT-1.5.3-3 is to be tested. Since we don't touch the stream, I
    // think we are fulfilling that requirement

    // MQTT-1.5.3-1
    #[test]
    fn check_simple_string() {
        let input = [0x00, 0x05, 0x41, 0xF0, 0xAA, 0x9B, 0x94];

        let s = mstring(&input);

        assert_eq!(
            s,
            Ok((
                &[][..],
                MString {
                    value: "A\u{2A6D4}"
                }
            ))
        )
    }

    #[tokio::test]
    async fn check_simple_string_roundtrip() {
        let input = [0x00, 0x05, 0x41, 0xF0, 0xAA, 0x9B, 0x94];

        let (_, s) = mstring(&input).unwrap();

        let mut vec = vec![];

        MString::write_to(&s, &mut Pin::new(&mut vec))
            .await
            .unwrap();

        assert_eq!(input, &vec[..])
    }

    // MQTT-1.5.3-2
    #[test]
    fn check_forbidden_characters() {
        let input = [0x00, 0x02, 0x00, 0x01];

        let s = mstring(&input);

        assert_eq!(
            s,
            Err(nom::Err::Error(nom::error::Error {
                input: &input[..],
                code: nom::error::ErrorKind::MapRes
            }))
        )
    }
}
