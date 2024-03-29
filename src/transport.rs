//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use futures::AsyncRead as FuturesAsyncRead;
use futures::AsyncWrite as FuturesAsyncWrite;
use tokio::io::AsyncRead as TokioAsyncRead;
use tokio::io::AsyncWrite as TokioAsyncWrite;
use tokio::io::DuplexStream;
use tokio::net::TcpStream;
use tokio_util::compat::Compat as TokioCompat;
use tokio_util::compat::TokioAsyncReadCompatExt;

pub(crate) enum MqttConnection {
    Tokio(TokioCompat<tokio::net::TcpStream>),
    Duplex(TokioCompat<tokio::io::DuplexStream>),
}

impl TokioAsyncRead for MqttConnection {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t.get_mut()).poll_read(cx, buf),
            MqttConnection::Duplex(d) => std::pin::pin!(d.get_mut()).poll_read(cx, buf),
        }
    }
}

impl TokioAsyncWrite for MqttConnection {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t.get_mut()).poll_write(cx, buf),
            MqttConnection::Duplex(d) => std::pin::pin!(d.get_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t.get_mut()).poll_flush(cx),
            MqttConnection::Duplex(d) => std::pin::pin!(d.get_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttConnection::Tokio(t) => std::pin::pin!(t.get_mut()).poll_shutdown(cx),
            MqttConnection::Duplex(d) => std::pin::pin!(d.get_mut()).poll_shutdown(cx),
        }
    }
}

impl FuturesAsyncRead for MqttConnection {
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

impl FuturesAsyncWrite for MqttConnection {
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
