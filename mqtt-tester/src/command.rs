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


    pub async fn wait_for_write<C>(mut self, commands: C) -> Result<std::process::Output, miette::Error>
        where C: IntoIterator<Item = ClientCommand>,
    {
        let mut client = self.inner
            .spawn()
            .into_diagnostic()?;

        let mut to_client = client.stdin.take().unwrap();
        let mut from_client = client.stdout.take().unwrap();

        for command in commands {
            match command {
                ClientCommand::Send(bytes) => to_client.write_all(&bytes).await.into_diagnostic()?,
                ClientCommand::WaitFor(expected_bytes) => {
                    let mut buf = Vec::with_capacity(expected_bytes.len());
                    match tokio::time::timeout(std::time::Duration::from_millis(100), from_client.read_exact(&mut buf)).await {
                        Ok(Ok(_)) => if buf != expected_bytes {
                            return Err(miette::miette!("Received Bytes did not match expected bytes: {:?} != {:?}", buf, expected_bytes))
                        },
                        Ok(Err(e)) => return Err(e).into_diagnostic(),
                        Err(_elapsed) => return Err(miette::miette!("Did not hear from server until timeout")),
                    }
                },
                ClientCommand::WaitAndCheck(check) => {
                    let mut buf = Vec::new();
                    match tokio::time::timeout(std::time::Duration::from_millis(100), from_client.read_to_end(&mut buf)).await {
                        Ok(Ok(_)) => {
                            if !check(&buf) {
                                return Err(miette::miette!("Check failed for Bytes {:?}", buf))
                            }
                        },
                        Ok(Err(e)) => return Err(e).into_diagnostic(),
                        Err(_elapsed) => return Err(miette::miette!("Did not hear from server until timeout")),
                    }
                }
            }
        }

        client.wait_with_output().await.into_diagnostic()
    }

}


