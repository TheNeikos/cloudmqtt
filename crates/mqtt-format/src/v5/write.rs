//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug)]
pub enum MqttWriteError {
    Invariant,
}

#[cfg_attr(test, allow(clippy::len_without_is_empty))]
pub trait WriteMqttPacket: Send {
    type Error: From<MqttWriteError>;

    fn write_byte(&mut self, u: u8) -> core::result::Result<(), Self::Error>;
    fn write_slice(&mut self, u: &[u8]) -> core::result::Result<(), Self::Error>;

    #[inline]
    fn write_u16(&mut self, u: u16) -> core::result::Result<(), Self::Error> {
        self.write_byte((u >> 8) as u8)?;
        self.write_byte(u as u8)
    }

    #[inline]
    fn write_u32(&mut self, u: u32) -> core::result::Result<(), Self::Error> {
        let bytes = u.to_be_bytes();
        self.write_byte(bytes[0])?;
        self.write_byte(bytes[1])?;
        self.write_byte(bytes[2])?;
        self.write_byte(bytes[3])
    }

    #[cfg(test)]
    fn len(&self) -> usize;
}

#[cfg(test)]
mod test {
    use super::WriteMqttPacket;
    use crate::v5::test::TestWriter;

    #[test]
    fn test_write_u16() {
        for i in (0..=u16::MAX).step_by(991) {
            let mut writer = TestWriter { buffer: Vec::new() };
            writer.write_u16(i).unwrap();
            assert_eq!(&writer.buffer, &i.to_be_bytes());
        }
    }

    #[test]
    fn test_write_u32_1() {
        let num = 1;
        let mut writer = TestWriter { buffer: Vec::new() };
        writer.write_u32(num).unwrap();
        assert_eq!(&writer.buffer, &[0x00, 0x00, 0x00, 0x01]);
        assert_eq!(&writer.buffer, &num.to_be_bytes());
    }

    #[test]
    fn test_write_u32_12() {
        let num = 12;
        let mut writer = TestWriter { buffer: Vec::new() };
        writer.write_u32(num).unwrap();
        assert_eq!(&writer.buffer, &[0x00, 0x00, 0x00, 12u8.to_be()]);
        assert_eq!(&writer.buffer, &num.to_be_bytes());
    }

    #[test]
    fn test_write_u32_range() {
        // step by some prime number
        for i in (0..268_435_455).step_by(991) {
            let mut writer = TestWriter { buffer: Vec::new() };
            writer.write_u32(i).unwrap();
            assert_eq!(&writer.buffer, &i.to_be_bytes());
        }
    }
}
