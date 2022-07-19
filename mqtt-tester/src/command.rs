use bytes::{BufMut, BytesMut};
use miette::IntoDiagnostic;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Command {
    inner: tokio::process::Command,
}

pub enum ClientCommand {
    Send(&'static [u8]),
    WaitFor(&'static [u8]),
    WaitAndCheck(Box<dyn FnOnce(&[u8]) -> bool>),
}

impl Command {
    pub fn new(inner: tokio::process::Command) -> Self {
        Self { inner }
    }

    pub async fn wait_for_write<C>(
        mut self,
        commands: C,
    ) -> Result<std::process::Output, miette::Error>
    where
        C: IntoIterator<Item = ClientCommand>,
    {
        let mut client = self.inner.spawn().into_diagnostic()?;

        let mut to_client = client.stdin.take().unwrap();
        let mut from_client = client.stdout.take().unwrap();

        for command in commands {
            match command {
                ClientCommand::Send(bytes) => {
                    to_client.write_all(&bytes).await.into_diagnostic()?
                }
                ClientCommand::WaitFor(expected_bytes) => {
                    let mut buf = Vec::with_capacity(expected_bytes.len());
                    match tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        from_client.read_exact(&mut buf),
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
                        Err(_elapsed) => {
                            return Err(miette::miette!("Did not hear from server until timeout"))
                        }
                    }
                }
                ClientCommand::WaitAndCheck(check) => {
                    match tokio::time::timeout(std::time::Duration::from_millis(100), async {
                        let mut buffer = BytesMut::new();
                        buffer.put_u16(from_client.read_u16().await.into_diagnostic()?);
                        buffer.put_u8(from_client.read_u8().await.into_diagnostic()?);

                        if buffer[1] & 0b1000_0000 != 0 {
                            buffer.put_u8(from_client.read_u8().await.into_diagnostic()?);
                            if buffer[2] & 0b1000_0000 != 0 {
                                buffer.put_u8(from_client.read_u8().await.into_diagnostic()?);
                                if buffer[3] & 0b1000_0000 != 0 {
                                    buffer.put_u8(from_client.read_u8().await.into_diagnostic()?);
                                }
                            }
                        }

                        let rest_len = buffer[1..].iter().enumerate().fold(0, |val, (exp, len)| {
                            val + (*len as u32 & 0b0111_1111) * 128u32.pow(exp as u32)
                        });

                        let mut rest_buf = buffer.limit(rest_len as usize);
                        from_client
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
                        Err(_elapsed) => {
                            return Err(miette::miette!("Did not hear from server until timeout"))
                        }
                    }
                }
            }
        }

        client.wait_with_output().await.into_diagnostic()
    }
}
