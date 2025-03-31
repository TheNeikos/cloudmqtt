//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use cloudmqtt::CloudmqttClient;
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let mut client = CloudmqttClient::new_with_address("localhost:1883".to_string()).await;

    let whatsub = client.subscribe("whats/up").await.unwrap();
    let morestuff = client.subscribe("more/stuff").await.unwrap();
    let mut combined = futures::stream::select(whatsub, morestuff);

    while let Some(next_message) = combined.next().await {
        println!("Got: {next_message:?}");
    }

    client.wait_for_shutdown().await;
}
