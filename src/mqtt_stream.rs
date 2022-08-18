//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use tokio::io::{AsyncRead, AsyncWrite};

pub enum MqttStream {
    UnsecuredTcp(tokio::net::TcpStream),
    MemoryDuplex(tokio::io::DuplexStream),
}

impl AsyncWrite for MqttStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            MqttStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_write(cx, buf),
            MqttStream::MemoryDuplex(duplex) => std::pin::Pin::new(duplex).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_flush(cx),
            MqttStream::MemoryDuplex(duplex) => std::pin::Pin::new(duplex).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_shutdown(cx),
            MqttStream::MemoryDuplex(duplex) => std::pin::Pin::new(duplex).poll_shutdown(cx),
        }
    }
}

impl AsyncRead for MqttStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MqttStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_read(cx, buf),
            MqttStream::MemoryDuplex(duplex) => std::pin::Pin::new(duplex).poll_read(cx, buf),
        }
    }
}
