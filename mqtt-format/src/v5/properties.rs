//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::Bytes;

use super::MResult;

pub(crate) struct MqttPropertySlot<T> {
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
    (
        $( packet_type: $packettypename:ident $(,)?)?
        $( anker: $anker:literal $(,)?)?
        pub struct $name:ident <$lt:lifetime> {
        $( $((anker: $prop_anker:literal ))? $prop_name:ident : $prop:ty),* $(,)?
    }) => {
        $(
            #[doc = std::concat!("Properties helper type for the [", std::stringify!($packettypename), "] type.")]
            #[doc = ""] // newline
        )?
        $(
            #[doc = $crate::v5::util::md_speclink!($anker)]
        )?
        #[derive(Debug)]
        pub struct $name < $lt > {
            $(
                $(
                    #[doc = $crate::v5::util::md_speclink!($prop_anker)]
                )?
                $prop_name: Option<$prop>
            ),*
        }

        impl<$lt> $name <$lt> {
            #[allow(dead_code)]
            pub(crate) fn new() -> Self {
                $name {
                    $($prop_name: None),*
                }
            }

            $(
                pub fn $prop_name(&self) -> Option<& $prop> {
                    self.$prop_name.as_ref()
                }
            )*

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
