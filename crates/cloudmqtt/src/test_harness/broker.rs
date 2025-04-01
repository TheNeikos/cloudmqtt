//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use dashmap::DashMap;
use tokio_util::codec::Framed;

use super::error::TestHarnessError;

#[derive(Default)]
pub(crate) struct Broker {
    name: String,
    connections: DashMap<String, Framed<tokio::io::DuplexStream, crate::codec::MqttPacketCodec>>,
}

impl std::fmt::Debug for Broker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Broker")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Broker {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connections: DashMap::new(),
        }
    }

    pub(crate) async fn connect(
        &self,
        client_name: String,
        connection: tokio::io::DuplexStream,
    ) -> Result<(), TestHarnessError> {
        let connection = Framed::new(connection, crate::codec::MqttPacketCodec);
        self.connections.insert(client_name, connection);
        Ok(())
    }
}
