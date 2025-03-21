//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use cloudmqtt::CloudmqttClient;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let mut client = CloudmqttClient::new("localhost:1883".to_string()).await;

    let mut multi_sub = client
        .subscription_builder()
        .with_subscription("whats/up")
        .with_subscription("more/things")
        .with_subscription("cloudmqtt/rocks")
        .build()
        .await;

    while let Some(next_message) = multi_sub.next().await {
        println!("Got: {next_message:?}");
    }

    client.wait_for_shutdown().await;
}
