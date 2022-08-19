//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use dashmap::DashMap;
use mqtt_format::v3::{connect_return::MConnectReturnCode, packet::MPacket, strings::MString};
use tokio::{
    io::DuplexStream,
    net::{TcpListener, ToSocketAddrs},
};

use crate::{error::MqttError, mqtt_stream::MqttStream, PacketIOError};

pub struct MqttServer {
    clients: DashMap<ClientId, ClientState>,
    client_source: ClientSource,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ClientId(String);

impl<'message> TryFrom<MString<'message>> for ClientId {
    type Error = ClientError;

    fn try_from(_ms: MString<'message>) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

#[derive(Debug, thiserror::Error)]
enum ClientError {
    #[error("An error occured during the handling of a packet")]
    Packet(#[from] PacketIOError),
}

struct ClientState {
    conn: Option<MqttStream>,
}

#[derive(Debug)]
enum ClientSource {
    UnsecuredTcp(TcpListener),
    Duplex(tokio::sync::mpsc::Receiver<DuplexStream>),
}

impl ClientSource {
    async fn accept(&mut self) -> Result<MqttStream, MqttError> {
        Ok({
            match self {
                ClientSource::UnsecuredTcp(listener) => listener
                    .accept()
                    .await
                    .map(|tpl| tpl.0)
                    .map(MqttStream::UnsecuredTcp)?,
                ClientSource::Duplex(recv) => recv
                    .recv()
                    .await
                    .map(MqttStream::MemoryDuplex)
                    .ok_or(MqttError::DuplexSourceClosed)?,
            }
        })
    }
}

impl MqttServer {
    pub async fn serve_v3_unsecured_tcp<Addr: ToSocketAddrs>(
        addr: Addr,
    ) -> Result<Self, MqttError> {
        let bind = TcpListener::bind(addr).await?;

        Ok(MqttServer {
            clients: DashMap::new(),
            client_source: ClientSource::UnsecuredTcp(bind),
        })
    }

    pub async fn accept_new_clients(&mut self) -> Result<(), MqttError> {
        loop {
            let client = self.client_source.accept().await?;
            if let Err(client_error) = self.accept_client(client).await {
                tracing::error!("Client error: {}", client_error)
            }
        }
    }

    async fn accept_client(&mut self, mut client: MqttStream) -> Result<(), ClientError> {
        let packet = crate::read_one_packet(&mut client).await?;

        if let MPacket::Connect {
            client_id,
            clean_session,
            ..
        } = packet.get_packet()?
        {
            let client_id = ClientId::try_from(client_id);
            let client_id: ClientId = { unimplemented!() };

            let session_present = if clean_session {
                let _ = self.clients.remove(&client_id);
                false
            } else {
                self.clients.contains_key(&client_id)
            };

            let conn_ack = MPacket::Connack {
                session_present,
                connect_return_code: MConnectReturnCode::Accepted,
            };

            crate::write_packet(&mut client, conn_ack).await?;

            {
                let state = self
                    .clients
                    .entry(client_id)
                    .or_insert_with(|| ClientState { conn: None });
                state.conn = Some(client);
            }
        } else {
        }

        Ok(())
    }
}
