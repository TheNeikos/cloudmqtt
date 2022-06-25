use super::{qos::MQualityOfService, strings::MString};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MLastWill<'message> {
    topic: MString<'message>,
    payload: &'message [u8],
    qos: MQualityOfService,
}
