//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::num::NonZeroU16;
use std::sync::Arc;

use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;
use tokio_util::codec::FramedRead;
use yoke::Yoke;

use super::InnerClient;
use crate::client::CallbackState;
use crate::client::Id;
use crate::codecs::MqttPacketCodec;
use crate::packets::MqttPacket;
use crate::packets::MqttWriter;
use crate::packets::StableBytes;
use crate::transport::MqttConnection;

pub(super) async fn handle_background_receiving(
    inner_clone: Arc<Mutex<InnerClient>>,
    mut conn_read: FramedRead<tokio::io::ReadHalf<MqttConnection>, MqttPacketCodec>,
    conn_read_sender: futures::channel::oneshot::Sender<
        FramedRead<tokio::io::ReadHalf<MqttConnection>, MqttPacketCodec>,
    >,
) -> Result<(), ()> {
    tracing::info!("Starting background task");
    let inner: Arc<Mutex<InnerClient>> = inner_clone;

    while let Some(next) = conn_read.next().await {
        let process_span = tracing::debug_span!(
            "Processing packet",
            packet_kind = tracing::field::Empty,
            packet_identifier = tracing::field::Empty
        );
        tracing::debug!(parent: &process_span, valid = next.is_ok(), "Received packet");
        let packet = match next {
            Ok(packet) => packet,
            Err(e) => panic!("Received err: {e}"),
        };
        process_span.record(
            "packet_kind",
            tracing::field::debug(packet.get().get_kind()),
        );

        match packet.get() {
            mqtt_format::v5::packets::MqttPacket::Auth(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Disconnect(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Pingreq(pingreq) => {
                handle_pingreq(pingreq, &inner, &process_span).await?
            }
            mqtt_format::v5::packets::MqttPacket::Pingresp(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Puback(mpuback) => {
                handle_puback(mpuback, &inner, &process_span, &packet).await?
            }
            mqtt_format::v5::packets::MqttPacket::Pubrec(pubrec) => {
                handle_pubrec(pubrec, &inner, &process_span, &packet).await?
            }
            mqtt_format::v5::packets::MqttPacket::Pubcomp(pubcomp) => {
                handle_pubcomp(pubcomp, &inner, &process_span, &packet).await?
            }
            mqtt_format::v5::packets::MqttPacket::Publish(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Pubrel(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Suback(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Unsuback(_) => todo!(),

            mqtt_format::v5::packets::MqttPacket::Connack(_)
            | mqtt_format::v5::packets::MqttPacket::Connect(_)
            | mqtt_format::v5::packets::MqttPacket::Subscribe(_)
            | mqtt_format::v5::packets::MqttPacket::Unsubscribe(_) => {
                todo!("Handle invalid packet")
            }
        }
    }

    tracing::debug!("Finished processing, returning reader");
    if let Err(conn_read) = conn_read_sender.send(conn_read) {
        tracing::error!("Failed to return reader");
        todo!()
    }

    Ok(())
}

async fn handle_pingreq(
    pingreq: &mqtt_format::v5::packets::pingreq::MPingreq,
    inner: &Arc<Mutex<InnerClient>>,
    process_span: &tracing::Span,
) -> Result<(), ()> {
    let mut inner = inner.lock().await;
    let inner = &mut *inner;
    let Some(ref mut conn_state) = inner.connection_state else {
        tracing::error!(parent: process_span, "No connection state found");
        todo!()
    };

    let packet = mqtt_format::v5::packets::MqttPacket::Pingresp(
        mqtt_format::v5::packets::pingresp::MPingresp,
    );
    conn_state.conn_write.send(packet).await.map_err(drop)?;

    Ok(())
}

async fn handle_pubcomp(
    pubcomp: &mqtt_format::v5::packets::pubcomp::MPubcomp<'_>,
    inner: &Arc<Mutex<InnerClient>>,
    process_span: &tracing::Span,
    packet: &MqttPacket,
) -> Result<(), ()> {
    match pubcomp.reason {
        mqtt_format::v5::packets::pubcomp::PubcompReasonCode::Success => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!(parent: process_span, "No session state found");
                todo!()
            };
            let pident = NonZeroU16::try_from(pubcomp.packet_identifier.0)
                .expect("zero PacketIdentifier not valid here");
            process_span.record("packet_identifier", pident);

            if session_state
                .outstanding_packets
                .exists_outstanding_packet(pident)
            {
                session_state.outstanding_packets.remove_by_id(pident);
                tracing::trace!(parent: process_span, "Removed packet id from outstanding packets");

                if let Some(callback) = inner
                    .outstanding_completions
                    .get_mut(&Id::PacketIdentifier(pident))
                {
                    match callback {
                        CallbackState::Qos2 { on_complete, .. } => {
                            if let Some(on_complete) = on_complete.take() {
                                if let Err(_) = on_complete.send(packet.clone()) {
                                    tracing::trace!("Could not send ack, receiver was dropped.")
                                }
                            } else {
                                todo!("Invariant broken: Double on_complete for a single pid: {pident}")
                            }
                        }
                        _ => todo!(),
                    }
                }
            }
        }
        _ => todo!("Handle errors"),
    }

    Ok(())
}

async fn handle_puback(
    mpuback: &mqtt_format::v5::packets::puback::MPuback<'_>,
    inner: &Arc<Mutex<InnerClient>>,
    process_span: &tracing::Span,
    packet: &MqttPacket,
) -> Result<(), ()> {
    match mpuback.reason {
        mqtt_format::v5::packets::puback::PubackReasonCode::Success
        | mqtt_format::v5::packets::puback::PubackReasonCode::NoMatchingSubscribers => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!(parent: process_span, "No session state found");
                todo!()
            };

            let pident = std::num::NonZeroU16::try_from(mpuback.packet_identifier.0)
                .expect("Zero PacketIdentifier not valid here");
            process_span.record("packet_identifier", pident);

            if session_state
                .outstanding_packets
                .exists_outstanding_packet(pident)
            {
                session_state.outstanding_packets.remove_by_id(pident);
                tracing::trace!(parent: process_span, "Removed packet id from outstanding packets");

                if let Some(callback) = inner
                    .outstanding_completions
                    .remove(&Id::PacketIdentifier(pident))
                {
                    match callback {
                        CallbackState::Qos1 { on_acknowledge } => {
                            if let Err(_) = on_acknowledge.send(packet.clone()) {
                                tracing::trace!("Could not send ack, receiver was dropped.")
                            }
                        }
                        _ => todo!(),
                    }
                }
            } else {
                tracing::error!(parent: process_span, "Packet id does not exist in outstanding packets");
                todo!()
            }

            // TODO: Forward mpuback.properties etc to the user
        }

        _ => todo!("Handle errors"),
    }

    Ok(())
}

async fn handle_pubrec(
    pubrec: &mqtt_format::v5::packets::pubrec::MPubrec<'_>,
    inner: &Arc<Mutex<InnerClient>>,
    process_span: &tracing::Span,
    packet: &MqttPacket,
) -> Result<(), ()> {
    match pubrec.reason {
        mqtt_format::v5::packets::pubrec::PubrecReasonCode::Success => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!(parent: process_span, "No session state found");
                todo!()
            };
            let Some(ref mut conn_state) = inner.connection_state else {
                tracing::error!(parent: process_span, "No session state found");
                todo!()
            };
            let pident = NonZeroU16::try_from(pubrec.packet_identifier.0)
                .expect("zero PacketIdentifier not valid here");
            process_span.record("packet_identifier", pident);

            if session_state
                .outstanding_packets
                .exists_outstanding_packet(pident)
            {
                let pubrel = mqtt_format::v5::packets::MqttPacket::Pubrel(
                    mqtt_format::v5::packets::pubrel::MPubrel {
                        packet_identifier: pubrec.packet_identifier,
                        reason: mqtt_format::v5::packets::pubrel::PubrelReasonCode::Success,
                        properties: mqtt_format::v5::packets::pubrel::PubrelProperties::new(),
                    },
                );

                let mut bytes = tokio_util::bytes::BytesMut::new();
                bytes.reserve(pubrel.binary_size() as usize);
                pubrel.write(&mut MqttWriter(&mut bytes)).map_err(drop)?;
                let pubrel_packet = MqttPacket {
                    packet: Yoke::try_attach_to_cart(StableBytes(bytes.freeze()), |bytes| {
                        mqtt_format::v5::packets::MqttPacket::parse_complete(bytes)
                    })
                    .unwrap(),
                };
                session_state
                    .outstanding_packets
                    .update_by_id(pident, pubrel_packet);
                tracing::trace!(parent: process_span, "Update packet from outstanding packets");
                conn_state.conn_write.send(pubrel).await.map_err(drop)?;

                if let Some(callback) = inner
                    .outstanding_completions
                    .get_mut(&Id::PacketIdentifier(pident))
                {
                    match callback {
                        CallbackState::Qos2 { on_receive, .. } => {
                            if let Some(on_receive) = on_receive.take() {
                                if let Err(_) = on_receive.send(packet.clone()) {
                                    tracing::trace!("Could not send ack, receiver was dropped.")
                                }
                            } else {
                                todo!("Invariant broken: Double on_receive for a single pid: {pident}")
                            }
                        }
                        _ => todo!(),
                    }
                }
            }
        }
        _ => todo!("Handle errors"),
    }

    Ok(())
}
