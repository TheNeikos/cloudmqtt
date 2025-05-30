//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
//! Handling of MQTT Properties that are present in some packets

use winnow::Bytes;
use winnow::error::ErrMode;
use winnow::error::ParserError;

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
                return Err(ErrMode::from_input(input));
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
            #[doc = core::concat!("Properties helper type for the [", core::stringify!($packettypename), "] type.")]
            #[doc = ""] // newline
        )?
        $(
            #[doc = $crate::v5::util::md_speclink!($anker)]
        )?
        #[cfg_attr(feature = "yoke", derive(yoke::Yokeable))]
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name < $lt > {
            $(
                $(
                    #[doc = $crate::v5::util::md_speclink!($prop_anker)]
                )?
                pub $prop_name: Option<$prop>
            ),*
        }

        impl<$lt> $name <$lt> {
            #[allow(clippy::new_without_default)]
            pub const fn new() -> Self {
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

                winnow::combinator::trace(stringify!($name), |input: &mut & $lt Bytes| {
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
                        return Err(ErrMode::from_input(
                            input,
                        ));
                    }


                    Ok( $name {
                        $($prop_name: $prop_name.take_inner()),*
                    })
                }).parse_next(input)
            }

            pub fn binary_size(&self) -> u32 {
                use crate::v5::variable_header::MqttProperties;

                let prop_size = 0
                    $(
                        + self.$prop_name.as_ref().map(|p| $crate::v5::integers::variable_u32_binary_size(<$prop>::IDENTIFIER) + p.binary_size()).unwrap_or(0)
                     )*
                    ;

                $crate::v5::integers::variable_u32_binary_size(prop_size) + prop_size
            }

            pub fn write<W: crate::v5::write::WriteMqttPacket>(&self, buffer: &mut W) -> Result<(), W::Error> {
                use crate::v5::variable_header::MqttProperties;

                #[cfg(test)]
                let start_len = buffer.len();

                let size = 0
                    $(
                        + self.$prop_name.as_ref().map(|p| $crate::v5::integers::variable_u32_binary_size(<$prop>::IDENTIFIER) + p.binary_size()).unwrap_or(0)
                     )*
                    ;
                $crate::v5::integers::write_variable_u32(buffer, size)?;

                $(
                    if let Some(prop) = self.$prop_name.as_ref() {
                        $crate::v5::integers::write_variable_u32(buffer, <$prop>::IDENTIFIER)?;
                        prop.write(buffer)?;
                    }
                )*

                #[cfg(test)]
                {
                    let total_len = buffer.len() - start_len;

                    debug_assert_eq!(total_len, self.binary_size() as usize, "We must write as much as we calculate in advance");
                }

                Ok(())
            }
        }
    };
}
pub(crate) use define_properties;
