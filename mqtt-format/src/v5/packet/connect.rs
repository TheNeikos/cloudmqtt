use winnow::Bytes;

use crate::v5::{fixed_header::QualityOfService, level::ProtocolLevel, MResult};

pub struct MControlVariableHeader<'i> {
    protocol_name: &'i str,
    protocol_level: ProtocolLevel,

    user_name_flag: bool,
    password_flag: bool,
    will_retain: bool,
    will_qos: QualityOfService,
    will_flag: bool,
    clean_start: bool,
}

impl<'i> MControlVariableHeader<'i> {
    pub fn parse(input: &mut &'i Bytes) -> MResult<MControlVariableHeader<'i>> {
        let protocol_name = crate::v5::strings::parse_string(input)?;
        let protocol_level = ProtocolLevel::parse(input)?;
        let connect_flags = winnow::binary::u8(input)?;

        let user_name_flag = 0b1000_0000 & connect_flags != 0;
        let password_flag = 0b0100_0000 & connect_flags != 0;
        let will_retain = 0b0010_0000 & connect_flags != 0;
        let will_qos = QualityOfService::from_byte((0b0001_1000 & connect_flags) >> 3, input)?;
        let will_flag = 0b0000_0100 & connect_flags != 0;
        let clean_start = 0b0000_0001 & connect_flags != 0;

        Ok(Self {
            protocol_name,
            protocol_level,

            user_name_flag,
            password_flag,
            will_retain,
            will_qos,
            will_flag,
            clean_start,
        })
    }
}
