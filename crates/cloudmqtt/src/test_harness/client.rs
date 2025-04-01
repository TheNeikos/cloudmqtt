//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use super::broker::Broker;
use super::error::TestHarnessError;

pub(crate) struct Client {
    client: Arc<crate::CloudmqttClient>,
    name: String,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Client {
    pub(crate) fn new(name: String) -> Self {
        Self {
            client: Arc::new(crate::CloudmqttClient::new()),
            name,
        }
    }

    pub(crate) async fn connect_to(&mut self, broker: &mut Broker) -> Result<(), TestHarnessError> {
        let (client, server) = tokio::io::duplex(100);

        self.client.connect(client).await.unwrap();
        broker.connect(self.name.clone(), server).await.unwrap();
        Ok(())
    }

    pub(crate) async fn publish(
        &mut self,
        payload: impl AsRef<[u8]>,
        topic: impl AsRef<str>,
    ) -> Result<(), TestHarnessError> {
        tracing::debug!(payload = ?payload.as_ref(), topic = ?topic.as_ref(), "Sending out payload on topic");
        self.client
            .publish(payload, topic)
            .await
            .map_err(TestHarnessError::Client)
    }
}
