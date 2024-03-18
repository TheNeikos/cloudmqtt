use winnow::{token::take_while, Bytes, Parser};

use super::IMResult;

pub fn parse_u16(input: &Bytes) -> IMResult<&Bytes, u16> {
    winnow::binary::u16(winnow::binary::Endianness::Big).parse_peek(input)
}

pub fn parse_u32(input: &Bytes) -> IMResult<&Bytes, u32> {
    winnow::binary::u32(winnow::binary::Endianness::Big).parse_peek(input)
}

pub fn parse_variable(input: &Bytes) -> IMResult<&Bytes, u32> {
    let var_bytes = (
        take_while(0..=3, |b| b & 0b1000_0000 != 0),
        winnow::binary::u8.verify(|b: &u8| b & 0b1000_0000 == 0),
    );
    let (input, bytes) = var_bytes.recognize().parse_peek(input)?;

    let mut output: u32 = 0;

    for (exp, val) in bytes.iter().enumerate() {
        output += (*val as u32 & 0b0111_1111) * 128u32.pow(exp as u32);
    }

    Ok((input, output))
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::integers::{parse_u16, parse_u32, parse_variable};

    #[test]
    fn check_integer_parsing() {
        let input = 15u16.to_be_bytes();
        assert_eq!(
            parse_u16(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 15)
        );

        let input = 42u32.to_be_bytes();
        assert_eq!(
            parse_u32(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 42)
        );
    }

    #[test]
    fn check_variable_integers() {
        let input = [0x0];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 0)
        );

        let input = [0x7F];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 127)
        );

        let input = [0x80, 0x01];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 128)
        );

        let input = [0xFF, 0x7F];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 16_383)
        );

        let input = [0x80, 0x80, 0x01];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 16_384)
        );

        let input = [0xFF, 0xFF, 0x7F];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 2_097_151)
        );

        let input = [0x80, 0x80, 0x80, 0x01];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 2_097_152)
        );

        let input = [0xFF, 0xFF, 0xFF, 0x7F];
        assert_eq!(
            parse_variable(Bytes::new(&input)).unwrap(),
            (Bytes::new(&[]), 268_435_455)
        );

        let input = [0xFF, 0xFF, 0xFF, 0x8F];
        parse_variable(Bytes::new(&input)).unwrap_err();
    }
}
