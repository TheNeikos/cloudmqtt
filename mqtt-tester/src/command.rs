//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use bytes::{BufMut, BytesMut};
use miette::IntoDiagnostic;
use mqtt_format::v3::packet::MPacket;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{ChildStdin, ChildStdout},
};

pub struct Command {
    inner: tokio::process::Command,
}

pub type CheckBytesFn = Box<dyn FnOnce(&[u8]) -> bool>;

impl Command {
    pub fn new(inner: tokio::process::Command) -> Self {
        Self { inner }
    }

    pub fn spawn(mut self) -> miette::Result<(tokio::process::Child, Input, Output)> {
        let mut client = self.inner.spawn().into_diagnostic()?;
        let to_client = client.stdin.take().unwrap();
        let stdout = client.stdout.take().unwrap();
        Ok((client, Input(to_client), Output { stdout }))
    }
}

pub struct Input(ChildStdin);

impl Input {
    pub async fn send(&mut self, bytes: &[u8]) -> miette::Result<()> {
        self.0.write_all(bytes).await.into_diagnostic()
    }

    pub async fn send_packet<'m, P>(&mut self, packet: P) -> miette::Result<()>
    where
        P: Into<MPacket<'m>>,
    {
        let mut buf = vec![];
        packet
            .into()
            .write_to(std::pin::Pin::new(&mut buf))
            .await
            .into_diagnostic()?;
        self.send(&buf).await
    }
}

pub struct Output {
    stdout: ChildStdout,
}

impl Output {
    pub async fn wait_for(&mut self, expected_bytes: &[u8]) -> miette::Result<()> {
        let mut buf = vec![0; expected_bytes.len()];
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            self.stdout.read_exact(&mut buf),
        )
        .await
        {
            Ok(Ok(_)) => {
                if buf != expected_bytes {
                    return Err(miette::miette!(
                        "Received Bytes did not match expected bytes: {:?} != {:?}",
                        buf,
                        expected_bytes
                    ));
                }
            }
            Ok(Err(e)) => return Err(e).into_diagnostic(),
            Err(_elapsed) => return Err(miette::miette!("Did not hear from server until timeout")),
        }
        Ok(())
    }

    pub async fn wait_for_packet<'m, P>(&mut self, packet: P) -> miette::Result<()>
    where
        P: Into<MPacket<'m>>,
    {
        let mut buf = vec![];
        packet
            .into()
            .write_to(std::pin::Pin::new(&mut buf))
            .await
            .into_diagnostic()?;
        self.wait_for(&buf).await
    }

    pub async fn wait_and_check(&mut self, check: CheckBytesFn) -> miette::Result<()> {
        match tokio::time::timeout(std::time::Duration::from_millis(100), async {
            let mut buffer = BytesMut::new();
            buffer.put_u16(self.stdout.read_u16().await.into_diagnostic()?);
            buffer.put_u8(self.stdout.read_u8().await.into_diagnostic()?);

            if buffer[1] & 0b1000_0000 != 0 {
                buffer.put_u8(self.stdout.read_u8().await.into_diagnostic()?);
                if buffer[2] & 0b1000_0000 != 0 {
                    buffer.put_u8(self.stdout.read_u8().await.into_diagnostic()?);
                    if buffer[3] & 0b1000_0000 != 0 {
                        buffer.put_u8(self.stdout.read_u8().await.into_diagnostic()?);
                    }
                }
            }

            let rest_len = buffer[1..].iter().enumerate().fold(0, |val, (exp, len)| {
                val + (*len as u32 & 0b0111_1111) * 128u32.pow(exp as u32)
            });

            let mut rest_buf = buffer.limit(rest_len as usize);
            self.stdout
                .read_buf(&mut rest_buf)
                .await
                .into_diagnostic()?;
            Ok::<_, miette::Error>(rest_buf.into_inner())
        })
        .await
        {
            Ok(Ok(buffer)) => {
                if !check(&buffer) {
                    return Err(miette::miette!("Check failed for Bytes {:?}", buffer));
                }
            }
            Ok(Err(e)) => return Err(e),
            Err(_elapsed) => return Err(miette::miette!("Did not hear from server until timeout")),
        }

        Ok(())
    }
}
