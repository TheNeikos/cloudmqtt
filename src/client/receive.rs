//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::sync::Arc;

use futures::lock::Mutex;
use futures::StreamExt;
use tokio_util::codec::FramedRead;
use tracing::Instrument;
use yoke::Yoke;

use super::InnerClient;
use crate::codecs::MqttPacketCodec;
use crate::packet_identifier::PacketIdentifier;
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

        tracing::trace!("Calling on_packet_recv() handler");
        (inner.lock().await.default_handlers.on_packet_recv)(packet.clone());

        match packet.get() {
            mqtt_format::v5::packets::MqttPacket::Auth(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Disconnect(_) => todo!(),
            mqtt_format::v5::packets::MqttPacket::Pingreq(pingreq) => {
                handle_pingreq(pingreq).instrument(process_span).await?
            }
            mqtt_format::v5::packets::MqttPacket::Pingresp(pingresp) => {
                handle_pingresp(pingresp, &inner)
                    .instrument(process_span)
                    .await?
            }
            mqtt_format::v5::packets::MqttPacket::Puback(_mpuback) => {
                handle_puback(&packet.try_into().unwrap(), &inner)
                    .instrument(process_span)
                    .await?
            }
            mqtt_format::v5::packets::MqttPacket::Pubrec(pubrec) => {
                handle_pubrec(pubrec, &inner, &packet)
                    .instrument(process_span)
                    .await?
            }
            mqtt_format::v5::packets::MqttPacket::Pubcomp(pubcomp) => {
                handle_pubcomp(pubcomp, &inner, &packet)
                    .instrument(process_span)
                    .await?
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
    if let Err(_conn_read) = conn_read_sender.send(conn_read) {
        tracing::error!("Failed to return reader");
        todo!()
    }

    Ok(())
}

async fn handle_pingresp(
    _pingresp: &mqtt_format::v5::packets::pingresp::MPingresp,
    inner: &Arc<Mutex<InnerClient>>,
) -> Result<(), ()> {
    let mut inner = inner.lock().await;
    let inner = &mut *inner;

    if let Some(cb) = inner.outstanding_callbacks.take_ping_req() {
        if cb.send(()).is_err() {
            tracing::debug!("PingReq completion handler was dropped before receiving response")
        }
    } else {
        tracing::warn!("Received an unwarranted PingResp from the server, continuing")
    }

    Ok(())
}

async fn handle_pingreq(_pingreq: &mqtt_format::v5::packets::pingreq::MPingreq) -> Result<(), ()> {
    tracing::warn!("Received an unwarranted PingReq from the server. This is unclear in the spec. Ignoring and continuing...");

    Ok(())
}

async fn handle_pubcomp(
    pubcomp: &mqtt_format::v5::packets::pubcomp::MPubcomp<'_>,
    inner: &Arc<Mutex<InnerClient>>,
    packet: &MqttPacket,
) -> Result<(), ()> {
    match pubcomp.reason {
        mqtt_format::v5::packets::pubcomp::PubcompReasonCode::Success => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!("No session state found");
                todo!()
            };
            let pident = PacketIdentifier::from(pubcomp.packet_identifier);
            tracing::Span::current().record("packet_identifier", tracing::field::display(pident));

            if session_state
                .outstanding_packets
                .exists_outstanding_packet(pident)
            {
                session_state.outstanding_packets.remove_by_id(pident);
                tracing::trace!("Removed packet id from outstanding packets");

                if let Some(callback) = inner.outstanding_callbacks.take_qos2_complete(pident) {
                    if callback.on_complete.send(packet.clone()).is_err() {
                        tracing::trace!("Could not send ack, receiver was dropped.")
                    }
                } else {
                    todo!("Invariant broken: Received on_complete for unknown packet")
                }
            }
        }
        _ => todo!("Handle errors"),
    }

    Ok(())
}

async fn handle_puback(
    puback: &crate::packets::Puback,
    inner: &Arc<Mutex<InnerClient>>,
) -> Result<(), ()> {
    tracing::trace!("Calling on_qos1_acknowledge handler");
    (inner.lock().await.default_handlers.on_qos1_acknowledge)(puback.clone());
    let mpuback = puback.get();

    match mpuback.reason {
        mqtt_format::v5::packets::puback::PubackReasonCode::Success
        | mqtt_format::v5::packets::puback::PubackReasonCode::NoMatchingSubscribers => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!("No session state found");
                todo!()
            };

            let pident = PacketIdentifier::from(mpuback.packet_identifier);
            tracing::Span::current().record("packet_identifier", tracing::field::display(pident));

            if session_state
                .outstanding_packets
                .exists_outstanding_packet(pident)
            {
                session_state.outstanding_packets.remove_by_id(pident);
                tracing::trace!("Removed packet id from outstanding packets");

                if let Some(callback) = inner.outstanding_callbacks.take_qos1(pident) {
                    if callback.on_acknowledge.send(puback.clone()).is_err() {
                        tracing::trace!("Could not send ack, receiver was dropped.")
                    }
                }
            } else {
                tracing::error!("Packet id does not exist in outstanding packets");
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
    packet: &MqttPacket,
) -> Result<(), ()> {
    match pubrec.reason {
        mqtt_format::v5::packets::pubrec::PubrecReasonCode::Success => {
            let mut inner = inner.lock().await;
            let inner = &mut *inner;
            let Some(ref mut session_state) = inner.session_state else {
                tracing::error!("No session state found");
                todo!()
            };
            let Some(ref mut conn_state) = inner.connection_state else {
                tracing::error!("No session state found");
                todo!()
            };
            let pident = PacketIdentifier::from(pubrec.packet_identifier);
            tracing::Span::current().record("packet_identifier", tracing::field::display(pident));

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
                tracing::trace!("Update packet from outstanding packets");
                conn_state.conn_write.send(pubrel).await.map_err(drop)?;

                if let Some(callback) = inner.outstanding_callbacks.take_qos2_receive(pident) {
                    if callback.on_receive.send(packet.clone()).is_err() {
                        tracing::trace!("Could not send ack, receiver was dropped.")
                    }
                } else {
                    todo!("Invariant broken: Receive PubRec for unawaited {pident}")
                }
            }
        }
        _ => todo!("Handle errors"),
    }

    Ok(())
}
