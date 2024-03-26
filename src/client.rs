//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::num::NonZeroU16;
use std::time::Duration;

use futures::AsyncRead;
use futures::AsyncWrite;
use tokio::io::DuplexStream;
use tokio::net::TcpStream;
use tokio_util::compat::Compat as TokioCompat;
use tokio_util::compat::TokioAsyncReadCompatExt;

enum MqttConnection {
    Tokio(TokioCompat<tokio::net::TcpStream>),
    Duplex(TokioCompat<tokio::io::DuplexStream>),
}

impl AsyncRead for MqttConnection {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t).poll_read(cx, buf),
            MqttConnection::Duplex(d) => std::pin::pin!(d).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MqttConnection {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t).poll_write(cx, buf),
            MqttConnection::Duplex(d) => std::pin::pin!(d).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t).poll_flush(cx),
            MqttConnection::Duplex(d) => std::pin::pin!(d).poll_flush(cx),
        }
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t).poll_close(cx),
            MqttConnection::Duplex(d) => std::pin::pin!(d).poll_close(cx),
        }
    }
}

pub enum MqttConnectTransport {
    TokioTcp(TcpStream),
    TokioDuplex(DuplexStream),
}

impl From<MqttConnectTransport> for MqttConnection {
    fn from(value: MqttConnectTransport) -> Self {
        match value {
            MqttConnectTransport::TokioTcp(t) => MqttConnection::Tokio(t.compat()),
            MqttConnectTransport::TokioDuplex(d) => MqttConnection::Duplex(d.compat()),
        }
    }
}

pub struct MinimalRequiredClientIdentifier(String);
pub struct PotentiallyAcceptedClientIdentifier(String);

pub enum ClientIdentifier {
    MinimalRequired(MinimalRequiredClientIdentifier),
    PotentiallyServerProvided,
    PotentiallyAccepted(PotentiallyAcceptedClientIdentifier),
}

impl ClientIdentifier {
    pub fn new_minimal_required(s: impl Into<String>) -> Result<ClientIdentifier, ()> {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }
}

pub enum CleanStart {
    No,
    Yes,
}

impl CleanStart {
    pub fn as_bool(&self) -> bool {
        match self {
            CleanStart::No => false,
            CleanStart::Yes => true,
        }
    }
}

pub enum KeepAlive {
    Disabled,
    Seconds(NonZeroU16),
}

impl KeepAlive {
    fn as_u16(&self) -> u16 {
        match self {
            KeepAlive::Disabled => 0,
            KeepAlive::Seconds(s) => s.get(),
        }
    }
}
impl TryFrom<Duration> for KeepAlive {
    type Error = ();
    fn try_from(value: Duration) -> Result<Self, Self::Error> {
        todo!()
    }
}

pub struct ConnectProperties {
    session_expiry_interval: Option<u32>,
}

impl ConnectProperties {
    fn new() -> Self {
        ConnectProperties {
            session_expiry_interval: None,
        }
    }
}

impl ConnectProperties {
    pub fn as_ref(&self) -> mqtt_format::v5::packets::connect::ConnectProperties<'_> {
        mqtt_format::v5::packets::connect::ConnectProperties {
            session_expiry_interval: self
                .session_expiry_interval
                .clone()
                .map(mqtt_format::v5::variable_header::SessionExpiryInterval),
            ..mqtt_format::v5::packets::connect::ConnectProperties::new()
        }
    }
}

pub struct MqttClientConnector {
    transport: MqttConnectTransport,
    client_identifier: ClientIdentifier,
    clean_start: CleanStart,
    keep_alive: KeepAlive,
    properties: ConnectProperties,
}

impl MqttClientConnector {
    pub fn set_session_expiry_inteveral(&mut self, interval: u32) {
        self.properties.session_expiry_interval = Some(interval);
    }
}

impl MqttClientConnector {
    pub fn new(
        transport: MqttConnectTransport,
        client_identifier: ClientIdentifier,
        clean_start: CleanStart,
        keep_alive: KeepAlive,
    ) -> MqttClientConnector {
        MqttClientConnector {
            transport,
            client_identifier,
            clean_start,
            keep_alive,
            properties: ConnectProperties::new(),
        }
    }

    pub async fn connect(self) -> Result<MqttClient, ()> {
        let conn: MqttConnection = self.transport.into();

        let conn_packet = mqtt_format::v5::packets::connect::MConnect {
            client_identifier: self.client_identifier.as_str(),
            username: None,
            password: None,
            clean_start: self.clean_start.as_bool(),
            will: None,
            properties: self.properties.as_ref(),
            keep_alive: self.keep_alive.as_u16(),
        };

        todo!()
    }
}

pub struct MqttClient {
    conn: MqttConnection,
}

impl MqttClient {
    pub(crate) fn new_with_connection(conn: MqttConnection) -> Self {
        MqttClient { conn }
    }
}
