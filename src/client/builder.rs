//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use futures::lock::Mutex;

use super::send::Callbacks;
use super::send::ClientHandlers;
use super::send::OnPacketRecvFn;
use super::send::OnPublishRecvFn;
use super::send::OnQos1AcknowledgeFn;
use super::InnerClient;
use super::MqttClient;

pub struct MqttClientBuilder {
    handlers: ClientHandlers,
}

impl MqttClientBuilder {
    pub(super) fn new() -> Self {
        Self {
            handlers: ClientHandlers::default(),
        }
    }

    pub fn with_on_packet_recv(mut self, f: OnPacketRecvFn) -> Self {
        self.handlers.on_packet_recv = f;
        self
    }

    pub fn with_on_publish_recv(mut self, f: OnPublishRecvFn) -> Self {
        self.handlers.on_publish_recv = f;
        self
    }

    pub fn with_handle_qos1_acknowledge(mut self, f: OnQos1AcknowledgeFn) -> Self {
        self.handlers.on_qos1_acknowledge = f;
        self
    }

    pub async fn build(self) -> Result<super::MqttClient, MqttClientBuilderError> {
        Ok({
            MqttClient {
                inner: Arc::new(Mutex::new(InnerClient {
                    connection_state: None,
                    session_state: None,
                    default_handlers: self.handlers,
                    outstanding_callbacks: Callbacks::new(),
                })),
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MqttClientBuilderError {}
