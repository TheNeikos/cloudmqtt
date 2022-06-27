use std::pin::Pin;

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
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, ToSocketAddrs},
};
use tokio_util::compat::TokioAsyncWriteCompatExt;

pub mod client_stream;
pub mod error;

pub struct MqttClient {
    client_stream: Option<client_stream::MqttClientStream>,
}

impl MqttClient {
    pub async fn connect_v3_unsecured<Addr: ToSocketAddrs>(
        addr: Addr,
        connection_params: MqttConnectionParams<'_>,
    ) -> Result<MqttClient, MqttError> {
        let mut stream = TcpStream::connect(addr).await?;

        let connect_packet = MPacket::Connect {
            protocol_name: MString { value: "MQTT" },
            protocol_level: 4,
            clean_session: connection_params.clean_session,
            will: connection_params.will,
            username: connection_params.username,
            password: connection_params.password,
            keep_alive: connection_params.keep_alive,
            client_id: connection_params.client_id,
        };

        {
            let mut buf = BufWriter::new((&mut stream).compat_write());
            connect_packet.write_to(Pin::new(&mut buf)).await?;
            buf.flush().await?;
        }

        Ok(MqttClient {
            client_stream: Some(MqttClientStream::UnsecuredTcp(stream)),
        })
    }

    pub async fn listen_for_message<'buffer>(
        &mut self,
        buffer: &'buffer mut Vec<u8>,
    ) -> Result<MPacket<'buffer>, MqttError> {
        let client_stream = match self.client_stream.as_mut() {
            Some(cs) => cs,
            None => return Err(MqttError::ConnectionClosed),
        };

        buffer.clear();
        buffer.resize(2, 0);
        client_stream.read_exact(&mut buffer[0..2]).await?;

        if buffer[1] & 0b1000_0000 != 0 {
            buffer.resize(buffer.len() + 1, 0);
            if buffer[2] & 0b1000_0000 != 0 {
                buffer.resize(buffer.len() + 1, 0);
                if buffer[3] & 0b1000_0000 != 0 {
                    buffer.resize(buffer.len() + 1, 0);
                }
            }
        }

        let bytes_needed = {
            match mfixedheader(&buffer) {
                Ok((&[], header)) => header.remaining_length,
                e => {
                    println!("Met an error while parsing fixed header: {:#?}", e);
                    return Err(MqttError::InvalidPacket);
                }
            }
        };

        println!("Reading {} more bytes", bytes_needed);

        buffer.resize(buffer.len() + bytes_needed as usize, 0);
        client_stream.read_exact(&mut buffer[2..]).await?;

        match mpacket(buffer) {
            Ok((&[], packet)) => Ok(packet),
            _ => Err(MqttError::InvalidPacket),
        }
    }

    pub async fn subscribe(
        &mut self,
        subscription_requests: &[MSubscriptionRequest<'_>],
    ) -> Result<(), MqttError> {
        let client_stream = match self.client_stream.as_mut() {
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

        {
            let mut buf = BufWriter::new(client_stream.compat_write());
            packet.write_to(Pin::new(&mut buf)).await?;
            buf.flush().await?;
        }

        Ok(())
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
