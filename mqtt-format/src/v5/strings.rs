use winnow::{
    binary::length_take,
    error::{ErrMode, FromExternalError},
    Bytes, Parser,
};

use super::{integers::parse_u16, MResult};

pub fn parse_string<'i>(input: &mut &'i Bytes) -> MResult<&'i str> {
    let maybe_str = length_take(parse_u16).parse_next(input)?;

    std::str::from_utf8(maybe_str)
        .map_err(|e| ErrMode::from_external_error(input, winnow::error::ErrorKind::Verify, e))
}

pub fn string_pair<'i>(input: &mut &'i Bytes) -> MResult<(&'i str, &'i str)> {
    let first = parse_string(input)?;
    let second = parse_string(input)?;

    Ok((first, second))
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::strings::parse_string;

    #[test]
    fn check_simple_string() {
        let input = [0x0, 0x5, 0x41, 0xF0, 0xAA, 0x9B, 0x94];

        assert_eq!(parse_string(&mut Bytes::new(&input)).unwrap(), "A𪛔");
    }
}
