//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use futures::AsyncRead;
use futures::AsyncWrite;
use tokio::io::DuplexStream;
use tokio::net::TcpStream;
use tokio_util::compat::Compat as TokioCompat;
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::client_identifier::ClientIdentifier;
use crate::keep_alive::KeepAlive;

pub(crate) enum MqttConnection {
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

crate::properties::define_properties! {
    properties_type: mqtt_format::v5::packets::connect::ConnectProperties,
    anker: "_Toc3901046",
    pub struct ConnectProperties {
        (anker: "_Toc3901048")
        session_expiry_interval: SessionExpiryInterval,

        (anker: "_Toc3901049")
        receive_maximum: ReceiveMaximum,

        (anker: "_Toc3901050")
        maximum_packet_size: MaximumPacketSize,

        (anker: "_Toc3901051")
        topic_alias_maximum: TopicAliasMaximum,

        (anker: "_Toc3901052")
        request_response_information: RequestResponseInformation,

        (anker: "_Toc3901053")
        request_problem_information: RequestProblemInformation,

        (anker: "_Toc3901054")
        user_properties: UserProperties<'a>,

        (anker: "_Toc3901055")
        authentication_method: AuthenticationMethod<'a>,

        (anker: "_Toc3901056")
        authentication_data: AuthenticationData<'a>,
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
    pub fn properties_mut(&mut self) -> &mut ConnectProperties {
        &mut self.properties
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
