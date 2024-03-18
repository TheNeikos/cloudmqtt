//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use cloudmqtt::server::handler::LoginError;
use cloudmqtt::server::handler::LoginHandler;
use cloudmqtt::server::handler::SubscriptionHandler;
use cloudmqtt::server::ClientId;
use cloudmqtt::server::MqttServer;
use mqtt_format::v3::qos::MQualityOfService;
use mqtt_format::v3::subscription_request::MSubscriptionRequest;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

struct SimpleLoginHandler;

#[async_trait::async_trait]
impl LoginHandler for SimpleLoginHandler {
    async fn allow_login(
        &self,
        client_id: Arc<ClientId>,
        username: Option<&str>,
        password: Option<&[u8]>,
    ) -> Result<(), LoginError> {
        info!("Client ({client_id:?}), tried to connect with username: {username:?}, password: {password:?}");

        if username == Some("foo") {
            return Err(LoginError::InvalidPassword);
        }

        Ok(())
    }
}

struct SimpleSubscriptionHandler;

#[async_trait::async_trait]
impl SubscriptionHandler for SimpleSubscriptionHandler {
    async fn allow_subscription(
        &self,
        _client_id: Arc<ClientId>,
        subscription: MSubscriptionRequest<'_>,
    ) -> Option<MQualityOfService> {
        if &*subscription.topic == "forbidden" {
            return None;
        }

        Some(subscription.qos)
    }
}

#[tokio::main]
async fn main() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_timer(tracing_subscriber::fmt::time::uptime());

    let filter_layer = tracing_subscriber::EnvFilter::from_default_env();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(filter_layer)
        .init();

    tracing::info!("Starting server");

    let server = MqttServer::serve_v3_unsecured_tcp("0.0.0.0:1883")
        .await
        .unwrap()
        .with_login_handler(SimpleLoginHandler)
        .with_subscription_handler(SimpleSubscriptionHandler);

    let server = server;

    tokio::spawn(server.subscribe_to_message(
        vec![String::from("foo/bar"), String::from("bar/#")],
        |msg| async move {
            info!("Got message: {msg:?}");
        },
    ));

    server.accept_new_clients().await.unwrap();
}
