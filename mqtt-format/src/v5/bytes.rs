use winnow::{binary::length_take, Bytes, Parser};

use super::MResult;

pub fn parse_data<'i>(input: &mut &'i Bytes) -> MResult<&'i [u8]> {
    length_take(super::integers::parse_u16).parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::bytes::parse_data;

    #[test]
    fn check_binary_data() {
        let input = &[0x0, 0x2, 0x4, 0x2];

        assert_eq!(parse_data(&mut Bytes::new(input)).unwrap(), &[0x4, 0x2]);
    }
}
