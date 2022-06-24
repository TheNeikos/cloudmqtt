use nom::{bytes::streaming::take, number::streaming::be_u16, IResult, Parser};
use nom_supreme::ParserExt;

/// A v3 MQTT string as defined in section 1.5.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MString<'message> {
    value: &'message str,
}

impl<'message> std::ops::Deref for MString<'message> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.value
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
    use super::{mstring, MString};

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
