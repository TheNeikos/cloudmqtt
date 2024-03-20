use winnow::{
    error::{ErrMode, ParserError},
    Bytes,
};

use super::MResult;

pub struct MqttPropertySlot<T> {
    allow_repeat: bool,
    slot: Option<T>,
}

impl<T> MqttPropertySlot<T> {
    pub(crate) const fn new(allow_repeat: bool) -> MqttPropertySlot<T> {
        MqttPropertySlot {
            allow_repeat,
            slot: None,
        }
    }

    pub(crate) fn use_slot(&mut self, input: &mut &Bytes, new_slot: T) -> MResult<()> {
        if self.slot.is_some() {
            if self.allow_repeat {
                return Ok(());
            } else {
                return Err(ErrMode::from_error_kind(
                    input,
                    winnow::error::ErrorKind::Verify,
                ));
            }
        } else {
            self.slot = Some(new_slot);
        }

        Ok(())
    }

    pub(crate) fn take_inner(self) -> Option<T> {
        self.slot
    }
}

macro_rules! define_properties {
    (pub struct $name:ident <$lt:lifetime> {
        $($prop_name:ident : $prop:ty),* $(,)?
    }) => {
        pub struct $name < $lt > {
            $($prop_name: Option<$prop>),*
        }

        impl<$lt> $name <$lt> {
            pub fn parse(input: &mut & $lt winnow::Bytes) -> crate::v5::MResult<$name<$lt>> {
                use winnow::Parser;
                use winnow::error::ErrMode;
                use winnow::error::ParserError;
                use winnow::stream::Stream;
                use $crate::v5::properties::MqttPropertySlot;

                $(
                    let mut $prop_name: MqttPropertySlot<$prop> = MqttPropertySlot::new(<$prop as crate::v5::variable_header::MqttProperties>::ALLOW_REPEATING);
                )*

                let mut properties_bytes = winnow::Bytes::new(winnow::binary::length_take(crate::v5::integers::parse_variable_u32).parse_next(input)?);

                while !properties_bytes.is_empty() {
                    let checkpoint = properties_bytes.checkpoint();
                    let id = crate::v5::integers::parse_variable_u32(&mut properties_bytes)?;

                    $(
                        if <$prop as crate::v5::variable_header::MqttProperties>::IDENTIFIER == id {
                            let slot = <$prop as crate::v5::variable_header::MqttProperties>::parse(&mut properties_bytes)?;
                            $prop_name.use_slot(&mut properties_bytes, slot)?;
                            continue
                        }
                    )*

                    input.reset(&checkpoint);
                    return Err(ErrMode::from_error_kind(
                        input,
                        winnow::error::ErrorKind::Verify,
                    ));
                }


                Ok( $name {
                    $($prop_name: $prop_name.take_inner()),*
                })
            }
        }
    };
}
pub(crate) use define_properties;
