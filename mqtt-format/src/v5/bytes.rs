//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::binary::length_take;
use winnow::Bytes;
use winnow::Parser;

use super::MResult;

pub fn parse_binary_data<'i>(input: &mut &'i Bytes) -> MResult<&'i [u8]> {
    winnow::combinator::trace("mqtt_binary_data", |input: &mut &'i Bytes| {
        length_take(super::integers::parse_u16).parse_next(input)
    })
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::Bytes;

    use crate::v5::bytes::parse_binary_data;

    #[test]
    fn check_binary_data() {
        let input = &[0x0, 0x2, 0x4, 0x2];

        assert_eq!(
            parse_binary_data(&mut Bytes::new(input)).unwrap(),
            &[0x4, 0x2]
        );
    }
}
