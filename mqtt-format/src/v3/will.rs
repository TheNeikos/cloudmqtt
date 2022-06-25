use super::{qos::MQualityOfService, strings::MString};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MLastWill<'message> {
    pub topic: MString<'message>,
    pub payload: &'message [u8],
    pub qos: MQualityOfService,
    pub retain: bool,
}
