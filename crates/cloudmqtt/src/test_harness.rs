//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct TestHarness {
    brokers: HashMap<String, Broker>,
    clients: HashMap<String, Client>,
}

impl TestHarness {
    pub fn start_broker(&mut self, name: String) -> Result<(), TestHarnessError> {
        self.brokers.insert(name, Broker::new());
        Ok(())
    }

    pub fn create_client(&mut self, name: String) -> Result<(), TestHarnessError> {
        self.clients.insert(name, Client::new());
        Ok(())
    }

    pub fn connect_client_to_broker(
        &mut self,
        client_name: String,
        broker_name: String,
    ) -> Result<(), TestHarnessError> {
        let broker = self
            .brokers
            .get_mut(&broker_name)
            .ok_or(TestHarnessError::BrokerNotFound(broker_name))?;
        let client = self
            .clients
            .get_mut(&client_name)
            .ok_or(TestHarnessError::ClientNotFound(client_name))?;

        client.connect_to(broker)
    }
}

#[derive(Debug, Default)]
struct Broker {}

impl Broker {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug, Default)]
struct Client {}

impl Client {
    fn new() -> Self {
        Self {}
    }

    fn connect_to(&mut self, _broker: &mut Broker) -> Result<(), TestHarnessError> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TestHarnessError {
    #[error("Broker '{}' not found", .0)]
    BrokerNotFound(String),

    #[error("Client '{}' not found", .0)]
    ClientNotFound(String),
}
