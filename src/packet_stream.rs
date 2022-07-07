//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::future::Ready;

use crate::{error::MqttError, MqttClient, MqttPacket};
use futures::Stream;
use mqtt_format::v3::{packet::MPacket, qos::MQualityOfService};
use tracing::{debug, error, trace};

pub struct Acknowledge;

pub struct PacketStreamBuilder<'client, ACK> {
    client: &'client MqttClient,
    ack_fn: ACK,
}

pub trait AckHandler: Send {
    type Future: std::future::Future<Output = Acknowledge> + Send;

    fn handle(&self, packet: MqttPacket) -> Self::Future;
}

impl<FUT, H> AckHandler for H
where
    FUT: std::future::Future<Output = Acknowledge> + Send,
    H: Send,
    H: for<'s> Fn(MqttPacket) -> FUT,
{
    type Future = FUT;

    fn handle(&self, packet: MqttPacket) -> Self::Future {
        (*self)(packet)
    }
}

pub struct NoOPAck;

impl AckHandler for NoOPAck {
    type Future = Ready<Acknowledge>;

    fn handle(&self, _packet: MqttPacket) -> Self::Future {
        std::future::ready(Acknowledge)
    }
}

impl<'client, HANDLER> PacketStreamBuilder<'client, HANDLER>
where
    HANDLER: AckHandler,
{
    pub(crate) fn new(client: &'client MqttClient) -> PacketStreamBuilder<'client, NoOPAck> {
        PacketStreamBuilder {
            client,
            ack_fn: NoOPAck,
        }
    }

    /// Set a custom acknowledge function
    ///
    /// This is useful if you want to do some work before acknowledging a given published message.
    ///
    ///
    ///
    /// However be careful about _how long these action can take_. This will delay any other
    /// messages that might need acknowledging and as such also pause the overall flow of the
    /// messages.
    ///
    /// The resulting packet stream will still be ordered in the same order as the packets have
    /// arrived.
    ///
    /// QoS 0 packets _cannot be acknowledged_ as such, this function will **not be called** for those.
    pub fn with_custom_ack_fn<NEWHANDLER: AckHandler>(
        self,
        f: NEWHANDLER,
    ) -> PacketStreamBuilder<'client, impl AckHandler> {
        PacketStreamBuilder {
            client: self.client,
            ack_fn: f,
        }
    }

    /// Construct the actual packet stream
    pub fn build(self) -> PacketStream<'client, HANDLER> {
        PacketStream {
            client: self.client,
            ack_fn: self.ack_fn,
        }
    }
}

pub struct PacketStream<'client, ACK: AckHandler> {
    client: &'client MqttClient,
    ack_fn: ACK,
}

impl<'client, ACK: AckHandler> PacketStream<'client, ACK> {
    pub fn stream(&self) -> impl Stream<Item = Result<MqttPacket, MqttError>> + '_ {
        futures::stream::try_unfold((), |()| async {
            let client = self.client;

            loop {
                let next_message = {
                    let mut mutex = client.client_receiver.lock().await;

                    let client_stream = match mutex.as_mut() {
                        Some(cs) => cs,
                        None => return Err(MqttError::ConnectionClosed),
                    };

                    MqttClient::read_one_packet(client_stream).await?
                };

                let packet = next_message.get_packet()?;
                match packet {
                    MPacket::Publish {
                        qos, id: Some(id), ..
                    } => {
                        match qos {
                            MQualityOfService::AtMostOnce => {}
                            MQualityOfService::AtLeastOnce => {
                                self.ack_fn.handle(next_message.clone());
                                // client
                                //     .received_packet_storage
                                //     .push_to_storage(next_message.clone());

                                let mut mutex = client.client_sender.lock().await;

                                let client_stream = match mutex.as_mut() {
                                    Some(cs) => cs,
                                    None => return Err(MqttError::ConnectionClosed),
                                };

                                MqttClient::acknowledge_packet(client_stream, packet).await?;
                            }
                            MQualityOfService::ExactlyOnce => {
                                if client.received_packets.contains(&id.0) {
                                    debug!(?packet, "Received duplicate packet");
                                    continue;
                                }

                                self.ack_fn.handle(next_message.clone());

                                trace!(?packet, "Inserting packet into received");
                                client.received_packets.insert(id.0);

                                let mut mutex = client.client_sender.lock().await;

                                let client_stream = match mutex.as_mut() {
                                    Some(cs) => cs,
                                    None => return Err(MqttError::ConnectionClosed),
                                };

                                MqttClient::acknowledge_packet(client_stream, packet).await?;
                            }
                        }
                    }
                    MPacket::Pubrel { id } => {
                        if client.received_packets.contains(&id.0) {
                            self.ack_fn.handle(next_message.clone());

                            let mut mutex = client.client_sender.lock().await;

                            let client_stream = match mutex.as_mut() {
                                Some(cs) => cs,
                                None => return Err(MqttError::ConnectionClosed),
                            };

                            MqttClient::acknowledge_packet(client_stream, packet).await?;
                        } else {
                            error!("Received a pubrel for a packet we did not expect");

                            return Ok(None);
                        }
                    }
                    _ => (),
                }

                return Ok(Some((next_message, ())));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use crate::{packet_stream::Acknowledge, MqttClient, MqttPacket};

    #[allow(unreachable_code, unused, clippy::diverging_sub_expression)]
    async fn check_making_stream_builder() {
        let client: MqttClient = todo!();

        // let builder = client.build_packet_stream().build();

        let builder = client
            .build_packet_stream()
            .with_custom_ack_fn(|packet| async move {
                println!("ACKing packet {packet:?}");
                Acknowledge
            })
            .build();

        let mut packet_stream = Box::pin(builder.stream());

        loop {
            while let Some(Ok(packet)) = packet_stream.next().await {
                let packet: MqttPacket = packet;
                println!("Received: {packet:#?}");
            }
        }
    }
}
