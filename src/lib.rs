//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::{pin::Pin, sync::Arc, time::Duration};

use bytes::{BufMut, Bytes, BytesMut};
use client_stream::MqttClientStream;
use dashmap::DashSet;
use error::MqttError;
use futures::{io::BufWriter, AsyncWriteExt};
use mqtt_format::v3::{
    header::mfixedheader,
    identifier::MPacketIdentifier,
    packet::{mpacket, MPacket},
    qos::MQualityOfService,
    strings::MString,
    subscription_request::{MSubscriptionRequest, MSubscriptionRequests},
    will::MLastWill,
};
use packet_stream::{NoOPAck, PacketStreamBuilder};
use tokio::{
    io::{AsyncReadExt, DuplexStream, ReadHalf, WriteHalf},
    net::{TcpStream, ToSocketAddrs},
    sync::Mutex,
};
use tokio_util::{compat::TokioAsyncWriteCompatExt, sync::CancellationToken};

use tracing::{debug, trace};

pub mod client_stream;
pub mod error;
pub mod packet_stream;

fn parse_packet(input: &[u8]) -> Result<MPacket<'_>, MqttError> {
    match nom::combinator::all_consuming(mpacket)(input) {
        Ok((_, packet)) => Ok(packet),

        Err(error) => {
            tracing::error!(?error, "Could not parse packet");
            Err(MqttError::InvalidPacket)
        }
    }
}

#[derive(Clone, Debug)]
pub struct MqttPacket {
    buffer: Bytes,
}

impl MqttPacket {
    pub(crate) fn new(buffer: Bytes) -> Self {
        Self { buffer }
    }

    pub fn get_packet(&self) -> Result<MPacket<'_>, MqttError> {
        parse_packet(&self.buffer)
    }
}

pub struct MqttClient {
    session_present: bool,
    client_receiver: Mutex<Option<ReadHalf<client_stream::MqttClientStream>>>,
    client_sender: Arc<Mutex<Option<WriteHalf<client_stream::MqttClientStream>>>>,
    received_packets: DashSet<u16>,
    keep_alive_duration: u16,
}

impl std::fmt::Debug for MqttClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttClient")
            .field("session_present", &self.session_present)
            .field("keep_alive_duration", &self.keep_alive_duration)
            .finish_non_exhaustive()
    }
}

macro_rules! write_packet {
    ($writer:expr, $packet:expr) => {
        async {
            let mut buf = BufWriter::new($writer.compat_write());
            let packet = $packet;
            trace!(?packet, "Sending packet");
            packet.write_to(Pin::new(&mut buf)).await?;
            buf.flush().await?;

            Ok::<(), MqttError>(())
        }
    };
}

impl MqttClient {
    async fn do_v3_connect(
        packet: MPacket<'_>,
        stream: MqttClientStream,
        keep_alive_duration: u16,
    ) -> Result<MqttClient, MqttError> {
        let (mut read_half, mut write_half) = tokio::io::split(stream);

        write_packet!(&mut write_half, packet).await?;

        let maybe_connect = MqttClient::read_one_packet(&mut read_half).await?;

        let session_present = match maybe_connect.get_packet()? {
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
            client_sender: Arc::new(Mutex::new(Some(write_half))),
            keep_alive_duration,
            received_packets: DashSet::new(),
        })
    }

    pub async fn connect_v3_duplex(
        duplex: DuplexStream,
        connection_params: MqttConnectionParams<'_>,
    ) -> Result<MqttClient, MqttError> {
        tracing::debug!("Connecting via duplex");
        let packet = connection_params.to_packet();

        MqttClient::do_v3_connect(
            packet,
            MqttClientStream::MemoryDuplex(duplex),
            connection_params.keep_alive,
        )
        .await
    }

    pub async fn connect_v3_unsecured_tcp<Addr: ToSocketAddrs>(
        addr: Addr,
        connection_params: MqttConnectionParams<'_>,
    ) -> Result<MqttClient, MqttError> {
        let stream = TcpStream::connect(addr).await?;

        tracing::debug!("Connected via TCP to {}", stream.peer_addr()?);

        let packet = connection_params.to_packet();

        trace!(?packet, "Connecting");

        MqttClient::do_v3_connect(
            packet,
            MqttClientStream::UnsecuredTcp(stream),
            connection_params.keep_alive,
        )
        .await
    }

    pub fn hearbeat(
        &self,
        _cancel_token: Option<CancellationToken>,
    ) -> impl std::future::Future<Output = Result<(), MqttError>> {
        let keep_alive_duration = self.keep_alive_duration;
        let sender = self.client_sender.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(
                    ((keep_alive_duration as u64 * 100) / 80).max(2),
                ))
                .await;

                let mut mutex = sender.lock().await;

                let mut client_stream = match mutex.as_mut() {
                    Some(cs) => cs,
                    None => return Err(MqttError::ConnectionClosed),
                };
                trace!("Sending hearbeat");

                let packet = MPacket::Pingreq;

                write_packet!(&mut client_stream, packet).await?;
            }
        }
    }

    async fn read_one_packet<W: tokio::io::AsyncRead + Unpin>(
        mut reader: W,
    ) -> Result<MqttPacket, MqttError> {
        debug!("Reading a packet");

        let mut buffer = BytesMut::new();
        buffer.put_u16(reader.read_u16().await?);

        trace!(
            "Packet has reported size on first byte: 0b{size:08b} = {size}",
            size = buffer[1] & 0b0111_1111
        );
        if buffer[1] & 0b1000_0000 != 0 {
            trace!("Reading one more byte from size");
            buffer.put_u8(reader.read_u8().await?);
            if buffer[2] & 0b1000_0000 != 0 {
                trace!("Reading one more byte from size");
                buffer.put_u8(reader.read_u8().await?);
                if buffer[3] & 0b1000_0000 != 0 {
                    trace!("Reading one more byte from size");
                    buffer.put_u8(reader.read_u8().await?);
                }
            }
        }

        trace!("Parsing fixed header");

        let (_, header) = mfixedheader(&buffer).map_err(|_| MqttError::InvalidPacket)?;

        trace!("Reading remaining length: {}", header.remaining_length);

        buffer.reserve(header.remaining_length as usize);
        let mut buffer = buffer.limit(header.remaining_length as usize);

        reader.read_buf(&mut buffer).await?;

        let packet = parse_packet(buffer.get_ref())?;

        trace!(?packet, "Received full packet");

        Ok(MqttPacket::new(buffer.into_inner().freeze()))
    }

    async fn acknowledge_packet<W: tokio::io::AsyncWrite + Unpin>(
        mut writer: W,
        packet: MPacket<'_>,
    ) -> Result<(), MqttError> {
        match packet {
            MPacket::Publish {
                qos: MQualityOfService::AtMostOnce,
                ..
            } => {}
            MPacket::Publish {
                id: Some(id),
                qos: qos @ MQualityOfService::AtLeastOnce,
                ..
            } => {
                trace!(?id, ?qos, "Acknowledging publish");

                let packet = MPacket::Puback { id };

                write_packet!(&mut writer, packet).await?;

                trace!(?id, "Acknowledged publish");
            }
            MPacket::Publish {
                id: Some(id),
                qos: qos @ MQualityOfService::ExactlyOnce,
                ..
            } => {
                trace!(?id, ?qos, "Acknowledging publish");

                let packet = MPacket::Pubrec { id };

                write_packet!(&mut writer, packet).await?;

                trace!(?id, "Acknowledged publish");
            }
            MPacket::Pubrel { id } => {
                trace!(?id, "Acknowledging pubrel");

                let packet = MPacket::Pubcomp { id };

                write_packet!(&mut writer, packet).await?;

                trace!(?id, "Acknowledged publish");
            }
            _ => panic!("Tried to acknowledge a non-publish packet"),
        };

        Ok(())
    }

    pub fn build_packet_stream(&self) -> PacketStreamBuilder<'_, NoOPAck> {
        PacketStreamBuilder::<NoOPAck>::new(self)
    }

    pub async fn subscribe(
        &self,
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
impl<'a> MqttConnectionParams<'a> {
    fn to_packet(&self) -> MPacket<'a> {
        MPacket::Connect {
            protocol_name: MString { value: "MQTT" },
            protocol_level: 4,
            clean_session: self.clean_session,
            will: self.will,
            username: self.username,
            password: self.password,
            keep_alive: self.keep_alive,
            client_id: self.client_id,
        }
    }
}
