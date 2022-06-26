#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("An IO Error occurred")]
    Io(#[from] std::io::Error),
    #[error("An error during writing to Buffer occurred")]
    Buffer(#[from] mqtt_format::v3::errors::MPacketWriteError),
    #[error("An invalid packet was received")]
    InvalidPacket,
    #[error("The connection was already closed")]
    ConnectionClosed,
}
