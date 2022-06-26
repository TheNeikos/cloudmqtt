use tokio::io::{AsyncRead, AsyncWrite};

pub enum MqttClientStream {
    UnsecuredTcp(tokio::net::TcpStream),
}

impl AsyncWrite for MqttClientStream {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match &mut *self {
            MqttClientStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttClientStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match &mut *self {
            MqttClientStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_shutdown(cx),
        }
    }
}

impl AsyncRead for MqttClientStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match &mut *self {
            MqttClientStream::UnsecuredTcp(direct) => std::pin::Pin::new(direct).poll_read(cx, buf),
        }
    }
}
