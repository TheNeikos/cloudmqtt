use super::errors::MPacketHeaderError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MQualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

pub fn mquality_of_service(lower: u8) -> Result<MQualityOfService, MPacketHeaderError> {
    match lower {
        0b00 => Ok(MQualityOfService::AtMostOnce),
        0b01 => Ok(MQualityOfService::AtLeastOnce),
        0b10 => Ok(MQualityOfService::ExactlyOnce),
        inv_qos => Err(MPacketHeaderError::InvalidQualityOfService(inv_qos)),
    }
}
