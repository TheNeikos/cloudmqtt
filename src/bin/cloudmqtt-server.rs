//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use cloudmqtt::server::handler::{LoginError, LoginHandler};
use cloudmqtt::server::{ClientId, MqttServer};
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
        .with_login_handler(SimpleLoginHandler);

    Arc::new(server).accept_new_clients().await.unwrap();
}
