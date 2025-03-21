//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::Bytes;
use winnow::error::FromExternalError;

use super::MResult;
use super::write::WResult;
use super::write::WriteMqttPacket;

#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QualityOfService {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

pub fn parse_qos(input: &mut &Bytes) -> MResult<QualityOfService> {
    winnow::binary::u8(input).and_then(|byte| {
        QualityOfService::try_from(byte)
            .map_err(|e| winnow::error::ErrMode::from_external_error(input, e))
    })
}

#[inline]
pub fn write_qos<W: WriteMqttPacket>(buffer: &mut W, qos: QualityOfService) -> WResult<W> {
    crate::v5::variable_header::write_u8(buffer, qos.into())
}

#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MaximumQualityOfService {
    AtMostOnce = 0,
    AtLeastOnce = 1,
}

pub fn parse_maximum_quality_of_service(input: &mut &Bytes) -> MResult<MaximumQualityOfService> {
    winnow::binary::u8(input).and_then(|byte| {
        MaximumQualityOfService::try_from(byte)
            .map_err(|e| winnow::error::ErrMode::from_external_error(input, e))
    })
}

#[inline]
pub fn write_maximum_quality_of_service<W: WriteMqttPacket>(
    buffer: &mut W,
    qos: MaximumQualityOfService,
) -> WResult<W> {
    crate::v5::variable_header::write_u8(buffer, qos.into())
}
