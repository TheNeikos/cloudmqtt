#[derive(Debug, thiserror::Error)]
pub enum MPacketHeaderError {
    #[error("An invalid Quality of Service (Qos) was supplied: {}", .0)]
    InvalidQualityOfService(u8),
    #[error("An invalid packet type was supplied: {}", .0)]
    InvalidPacketType(u8),
    #[error("The DUP flag was set in a publish message of Quality of Service (QoS) level 0.")]
    InvalidDupFlag,
    #[error("The packet length does not fit the remaining length")]
    InvalidPacketLength,
    #[error("The client sent an unsupported protocol name: {}", .0)]
    InvalidProtocolName(String),
    #[error("The client sent an unsupported protocol level: {}", .0)]
    InvalidProtocolLevel(u8),
    #[error("Received a forbidden reserved value")]
    ForbiddenReservedValue,
    #[error("Received an invalid connect return code in CONNACK")]
    InvalidConnectReturnCode(u8),
}
