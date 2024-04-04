//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

pub mod connect;
mod receive;
pub mod send;
mod state;

use std::sync::Arc;

use futures::lock::Mutex;

use self::send::Acknowledge;
use self::send::Callbacks;
use self::send::ClientHandlers;
use self::state::ConnectState;
use self::state::SessionState;

struct InnerClient {
    connection_state: Option<ConnectState>,
    session_state: Option<SessionState>,
    default_handlers: ClientHandlers,
    outstanding_callbacks: Callbacks,
}

pub struct MqttClient {
    inner: Arc<Mutex<InnerClient>>,
}

impl MqttClient {
    pub fn new_with_default_handlers() -> MqttClient {
        MqttClient {
            inner: Arc::new(Mutex::new(InnerClient {
                connection_state: None,
                session_state: None,
                default_handlers: ClientHandlers {
                    on_packet_recv: Box::new(|_| ()),
                    handle_acknowledge: Box::new(|_| Acknowledge::Yes),
                },
                outstanding_callbacks: Callbacks::new(),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::ClientHandlers;
    use crate::client::MqttClient;

    static_assertions::assert_impl_all!(MqttClient: Send, Sync);
    static_assertions::assert_impl_all!(ClientHandlers: Send);
}
