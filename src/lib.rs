use std::pin::Pin;

use bytes::{BufMut, Bytes, BytesMut};
use client_stream::MqttClientStream;
use error::MqttError;
use futures::{io::BufWriter, AsyncWriteExt};
use mqtt_format::v3::{
    header::mfixedheader,
    identifier::MPacketIdentifier,
    packet::{mpacket, MPacket},
    strings::MString,
    subscription_request::{MSubscriptionRequest, MSubscriptionRequests},
    will::MLastWill,
};
use packet_storage::PacketStorage;
use packet_stream::{NoOPAck, PacketStreamBuilder};
use tokio::{
    io::{AsyncReadExt, ReadHalf, WriteHalf},
    net::{TcpStream, ToSocketAddrs},
    sync::{Mutex, MutexGuard},
};
use tokio_util::{compat::TokioAsyncWriteCompatExt, sync::CancellationToken};

pub use packet_storage::MqttPacket;

pub mod client_stream;
pub mod error;
mod packet_storage;
pub mod packet_stream;

fn parse_packet(input: &[u8]) -> Result<MPacket<'_>, MqttError> {
    match mpacket(input) {
        Ok((&[], packet)) => Ok(packet),
        _ => Err(MqttError::InvalidPacket),
    }
}

struct PacketChannel {
    sender: tokio::sync::mpsc::Sender<MqttPacket>,
    receiver: Mutex<tokio::sync::mpsc::Receiver<MqttPacket>>,
}

impl PacketChannel {
    async fn receiver<'channel>(
        &'channel self,
    ) -> MutexGuard<'channel, tokio::sync::mpsc::Receiver<MqttPacket>> {
        self.receiver.lock().await
    }
}

impl PacketChannel {
    fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(4);
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }
}

pub struct MqttClient {
    session_present: bool,
    client_receiver: Mutex<Option<ReadHalf<client_stream::MqttClientStream>>>,
    client_sender: Mutex<Option<WriteHalf<client_stream::MqttClientStream>>>,
    received_packet_storage: PacketStorage,
    sent_packet_storage: PacketStorage,
    packet_channel: PacketChannel,
}

macro_rules! write_packet {
    ($writer:expr, $packet:expr) => {
        async {
            let mut buf = BufWriter::new($writer.compat_write());
            $packet.write_to(Pin::new(&mut buf)).await?;
            buf.flush().await?;

            Ok::<(), MqttError>(())
        }
    };
}

impl MqttClient {
    pub async fn connect_v3_unsecured<Addr: ToSocketAddrs>(
        addr: Addr,
        connection_params: MqttConnectionParams<'_>,
    ) -> Result<MqttClient, MqttError> {
        let stream = TcpStream::connect(addr).await?;

        let packet = MPacket::Connect {
            protocol_name: MString { value: "MQTT" },
            protocol_level: 4,
            clean_session: connection_params.clean_session,
            will: connection_params.will,
            username: connection_params.username,
            password: connection_params.password,
            keep_alive: connection_params.keep_alive,
            client_id: connection_params.client_id,
        };

        let (mut read_half, mut write_half) =
            tokio::io::split(MqttClientStream::UnsecuredTcp(stream));

        write_packet!(&mut write_half, packet).await?;

        let maybe_connect = MqttClient::read_one_packet(&mut read_half).await?;

        let session_present = match parse_packet(&maybe_connect)? {
            MPacket::Connack {
                session_present,
                connect_return_code,
            } => match connect_return_code {
                mqtt_format::v3::connect_return::MConnectReturnCode::Accepted => session_present,
                code => return Err(MqttError::ConnectionRejected(code)),
            },
            _ => return Err(MqttError::InvalidConnectionResponse),
        };

        Ok(MqttClient {
            session_present,
            client_receiver: Mutex::new(Some(read_half)),
            client_sender: Mutex::new(Some(write_half)),
            sent_packet_storage: PacketStorage::new(),
            received_packet_storage: PacketStorage::new(),
            packet_channel: PacketChannel::new(),
        })
    }

    async fn read_one_packet<W: tokio::io::AsyncRead + Unpin>(
        mut reader: W,
    ) -> Result<Bytes, MqttError> {
        let mut buffer = BytesMut::new();
        buffer.put_u16(reader.read_u16().await?);

        if buffer[1] & 0b1000_0000 != 0 {
            buffer.put_u8(reader.read_u8().await?);
            if buffer[2] & 0b1000_0000 != 0 {
                buffer.put_u8(reader.read_u8().await?);
                if buffer[3] & 0b1000_0000 != 0 {
                    buffer.put_u8(reader.read_u8().await?);
                }
            }
        }

        let (_, header) = mfixedheader(&buffer).map_err(|_| MqttError::InvalidPacket)?;

        let mut buffer = buffer.limit(header.remaining_length as usize);

        reader.read_buf(&mut buffer).await?;

        let _ = mpacket(buffer.get_ref()).map_err(|_| MqttError::InvalidPacket)?;
        Ok(buffer.into_inner().freeze())
    }

    pub fn build_packet_stream(&self) -> PacketStreamBuilder<'_, NoOPAck> {
        PacketStreamBuilder::<NoOPAck>::new(self)
    }

    /// Start listening for packets
    ///
    /// This future will not return until either the network stream
    /// is broken or the `cancel`lation token is cancelled.
    /// If this future is dropped then the client connection is severed
    /// and will have to be re-established with [`MqttClient::reconnect`]
    ///
    /// # Cancel safety
    ///
    /// This method is cancel-safe but leaves the connection open.
    /// If you wish to stop the MqttClient, you can do so by cancelling the given [`CancellationToken`].
    pub async fn listen(&self, cancel: Option<CancellationToken>) -> Result<(), MqttError> {
        let mut mutex = match self.client_receiver.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(MqttError::AlreadyListening),
        };

        let client_stream = match mutex.as_mut() {
            Some(cs) => cs,
            None => return Err(MqttError::ConnectionClosed),
        };

        let cancel = cancel.unwrap_or_default();

        loop {
            let packet_reader = async {
                let mut buffer = BytesMut::new();
                buffer.put_u16(client_stream.read_u16().await?);

                if buffer[1] & 0b1000_0000 != 0 {
                    buffer.put_u8(client_stream.read_u8().await?);
                    if buffer[2] & 0b1000_0000 != 0 {
                        buffer.put_u8(client_stream.read_u8().await?);
                        if buffer[3] & 0b1000_0000 != 0 {
                            buffer.put_u8(client_stream.read_u8().await?);
                        }
                    }
                }

                let (_, header) = mfixedheader(&buffer).map_err(|_| MqttError::InvalidPacket)?;

                let mut buffer = buffer.limit(header.remaining_length as usize);

                client_stream.read_buf(&mut buffer).await?;

                let _ = mpacket(buffer.get_ref()).map_err(|_| MqttError::InvalidPacket)?;

                Ok::<Bytes, MqttError>(buffer.into_inner().freeze())
            };

            tokio::select! {
                _ = cancel.cancelled() => break,
                buffer = packet_reader => {
                    let buffer = buffer?;
                    let mqttpacket = MqttPacket::new(buffer);
                    self.packet_channel.sender.send(mqttpacket).await.expect("Could not send out packet id");
                }
            }
        }
        Ok(())
    }

    pub async fn next_message(&self) -> Option<MqttPacket> {
        let mut receiver = self.packet_channel.receiver().await;

        receiver.recv().await
    }

    // pub async fn listen_for_message<'buffer>(&'buffer self) -> Result<MPacket<'buffer>, MqttError> {
    //     let mut client_stream = match self.client_receiver.try_lock().map(|mut cs| cs.take()) {
    //         Ok(Some(cs)) => cs,
    //         Ok(None) => return Err(MqttError::ConnectionClosed),
    //         Err(_) => return Err(MqttError::AlreadyListening),
    //     };
    //     let mut buffer = vec![2; 0];
    //     client_stream.read_exact(&mut buffer[0..2]).await?;

    //     if buffer[1] & 0b1000_0000 != 0 {
    //         buffer.push(client_stream.read_u8().await?);
    //         if buffer[2] & 0b1000_0000 != 0 {
    //             buffer.push(client_stream.read_u8().await?);
    //             if buffer[3] & 0b1000_0000 != 0 {
    //                 buffer.push(client_stream.read_u8().await?);
    //             }
    //         }
    //     }

    //     let bytes_needed = {
    //         match mfixedheader(&buffer) {
    //             Ok((&[], header)) => header.remaining_length,
    //             e => {
    //                 println!("Met an error while parsing fixed header: {:#?}", e);
    //                 return Err(MqttError::InvalidPacket);
    //             }
    //         }
    //     };

    //     println!("Reading {} more bytes", bytes_needed);

    //     buffer.resize(buffer.len() + bytes_needed as usize, 0);
    //     client_stream.read_exact(&mut buffer[2..]).await?;

    //     match mpacket(&buffer) {
    //         Ok((&[], packet)) => {
    //             *self.client_receiver.lock().await = Some(client_stream);
    //             Ok(packet)
    //         }
    //         _ => Err(MqttError::InvalidPacket),
    //     }
    // }

    pub async fn subscribe(
        &mut self,
        subscription_requests: &[MSubscriptionRequest<'_>],
    ) -> Result<(), MqttError> {
        let mut mutex = match self.client_sender.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err(MqttError::AlreadyListening),
        };

        let stream = match mutex.as_mut() {
            Some(cs) => cs,
            None => return Err(MqttError::ConnectionClosed),
        };

        let mut requests = vec![];
        for req in subscription_requests {
            req.write_to(&mut Pin::new(&mut requests)).await?;
        }

        let packet = MPacket::Subscribe {
            id: MPacketIdentifier(2),
            subscriptions: MSubscriptionRequests {
                count: subscription_requests.len(),
                data: &requests,
            },
        };

        write_packet!(stream, packet).await?;

        Ok(())
    }

    /// Checks whether a session was present upon connecting
    ///
    /// Note: This only reflects the presence of the session on connection.
    /// Later subscriptions or other commands that change the session do not
    /// update this value.
    pub fn session_present_at_connection(&self) -> bool {
        self.session_present
    }
}

pub struct MqttConnectionParams<'conn> {
    pub clean_session: bool,
    pub will: Option<MLastWill<'conn>>,
    pub username: Option<MString<'conn>>,
    pub password: Option<&'conn [u8]>,
    pub keep_alive: u16,
    pub client_id: MString<'conn>,
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use crate::MqttClient;

    assert_impl_all!(MqttClient: Send, Sync);
}
