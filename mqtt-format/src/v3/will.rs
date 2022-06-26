use super::{qos::MQualityOfService, strings::MString};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MLastWill<'message> {
    pub topic: MString<'message>,
    pub payload: &'message [u8],
    pub qos: MQualityOfService,
    pub retain: bool,
}

impl<'message> MLastWill<'message> {
    pub fn get_len(&self) -> usize {
        MString::get_len(&self.topic) + (2 + self.payload.len())
    }
}
