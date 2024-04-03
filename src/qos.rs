//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum QualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl From<QualityOfService> for mqtt_format::v5::qos::QualityOfService {
    fn from(value: QualityOfService) -> Self {
        match value {
            QualityOfService::AtMostOnce => mqtt_format::v5::qos::QualityOfService::AtMostOnce,
            QualityOfService::AtLeastOnce => mqtt_format::v5::qos::QualityOfService::AtLeastOnce,
            QualityOfService::ExactlyOnce => mqtt_format::v5::qos::QualityOfService::ExactlyOnce,
        }
    }
}
