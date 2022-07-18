use miette::IntoDiagnostic;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct Command {
    inner: tokio::process::Command,
}


impl Command {
    pub fn new(inner: tokio::process::Command) -> Self {
        Self { inner }
    }


    pub async fn wait_for_write<W, B>(mut self, writes: W) -> Result<std::process::Output, miette::Error>
        where W: IntoIterator<Item = B>,
              B: Iterator<Item = &'static u8>
    {
        let mut client = self.inner
            .spawn()
            .into_diagnostic()?;

        let mut to_client = client.stdin.take().unwrap();
        let mut from_client = client.stdout.take().unwrap();

        tokio::spawn(async move { from_client.read_to_end(&mut vec![]).await });

        for write in writes {
            let buf = write.copied().collect::<Vec<u8>>();
            to_client.write_all(&buf).await.into_diagnostic()?;
        }

        client.wait_with_output().await.into_diagnostic()
    }

}


