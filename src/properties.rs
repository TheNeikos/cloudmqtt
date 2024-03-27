//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

pub type TypeOfProperty<P> = <P as FormatProperty>::Inner;

macro_rules! define_properties {
    (@no_lt $name:ident $pat:ident $lt:lifetime) => {
       type $name<'a> = mqtt_format::v5::variable_header:: $pat <'a>;
    };
    (@no_lt $name:ident $pat:ident) => {
       type $name = mqtt_format::v5::variable_header:: $pat;
    };

    (@statify $pat:ident $lt:lifetime) => {
       mqtt_format::v5::variable_header:: $pat <'static>
    };
    (@statify $pat:ident) => {
        mqtt_format::v5::variable_header:: $pat
    };
    (
        properties_type: $packettypename:ty,
        $( anker: $anker:literal $(,)?)?
        pub struct $name:ident {
            $( $((anker: $prop_anker:literal ))? $prop_name:ident : $prop:ident $(<$prop_lt:lifetime>)? with setter = $setter:ty),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            $(
                pub(crate) $prop_name: Option<crate::properties::TypeOfProperty<crate::properties::define_properties!(@statify $prop $($prop_lt)?)>>
            ),*
        }

        paste::paste! {
            #[allow(dead_code)]
            impl $name {
                #[allow(clippy::new_without_default)]
                pub fn new() -> Self {
                    $name {
                        $($prop_name: None),*
                    }
                }

                $(
                    #[doc = core::concat!("Set the ", stringify!($prop_name), " property.") ]
                    $( #[doc = core::concat!("See also: ", crate::util::md_speclink!($prop_anker)) ] )?
                    pub fn [<with_ $prop_name>](&mut self, value: $setter) -> &mut Self {
                        <crate::properties::define_properties!(@statify $prop $($prop_lt)?) as crate::properties::FormatProperty>::apply(&mut self.$prop_name, value);

                        self
                    }
                )*

                pub fn as_ref(&self) -> $packettypename {
                    $packettypename {
                        $($prop_name: {
                            self.$prop_name.as_ref().map(|v| {
                                crate::properties::define_properties!(@no_lt Prop $prop $($prop_lt)?);
                                Prop {
                                    0: <crate::properties::define_properties!(@statify $prop $($prop_lt)?) as crate::properties::FormatProperty>::get(v)
                                }
                            })
                        }),*
                    }
                }
            }
        }
    };
}
pub(crate) use define_properties;

use crate::packets::VecWriter;
use crate::string::MqttString;

pub struct UserProperty {
    key: MqttString,
    value: MqttString,
}

pub(crate) trait FormatProperty {
    type Inner;
    type Setter;
    type Outer<'a>;

    fn apply(inner: &mut Option<Self::Inner>, value: impl Into<Self::Setter>);

    fn get(inner: &Self::Inner) -> Self::Outer<'_>;
}

impl<'i> FormatProperty for mqtt_format::v5::variable_header::UserProperties<'i> {
    type Inner = Vec<u8>;
    type Setter = UserProperty;
    type Outer<'a> = &'a [u8];

    fn apply(inner: &mut Option<Self::Inner>, key_value: impl Into<Self::Setter>) {
        let key_value = key_value.into();
        let user_prop = mqtt_format::v5::variable_header::UserProperty {
            key: key_value.key.as_ref(),
            value: key_value.value.as_ref(),
        };
        let inner = inner.get_or_insert_with(Default::default);
        if !inner.is_empty() {
            mqtt_format::v5::integers::write_variable_u32(
                &mut VecWriter(inner),
                <mqtt_format::v5::variable_header::UserProperties as mqtt_format::v5::variable_header::MqttProperties>::IDENTIFIER,
            )
            .expect("Writing a u32 should not fail")
        }
        user_prop
            .write(&mut VecWriter(inner))
            .expect("Writing MqttStrings should never be invalid");
    }

    fn get(inner: &Self::Inner) -> Self::Outer<'_> {
        inner.as_ref()
    }
}

macro_rules! define_property_types {
    (@access_pattern ref $value:ident) => {
        $value.as_ref()
    };
    (@access_pattern deref $value:ident) => {
        *$value
    };
    ([ $( $prop:ty => inner = $inner:ty; setter = $setter:ty; outer $mode:tt = $outer:ty ),* $(,)? ]) => {
        $(
            impl<'i> FormatProperty for $prop {
                type Inner = $inner;
                type Setter = $setter;
                type Outer<'a> = $outer;

                fn apply(inner: &mut Option<Self::Inner>, value: impl Into<Self::Setter>) {
                    *inner = Some(value.into());
                }

                fn get(inner: &Self::Inner) -> Self::Outer<'_> {
                    define_property_types!(@access_pattern $mode inner)
                }
            }
        )*
    };
}

define_property_types! {[
    mqtt_format::v5::variable_header::PayloadFormatIndicator => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::MessageExpiryInterval => inner = u32; setter = u32; outer deref = u32,
    mqtt_format::v5::variable_header::ContentType<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::ResponseTopic<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::CorrelationData<'i> => inner = Vec<u8>; setter = Vec<u8>; outer ref = &'a [u8],
    mqtt_format::v5::variable_header::SubscriptionIdentifier => inner = u32; setter = u32; outer deref = u32,
    mqtt_format::v5::variable_header::SessionExpiryInterval => inner = u32; setter = u32; outer deref = u32,
    mqtt_format::v5::variable_header::AssignedClientIdentifier<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::ServerKeepAlive => inner = u16; setter = u16; outer deref = u16,
    mqtt_format::v5::variable_header::AuthenticationMethod<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::AuthenticationData<'i> => inner = Vec<u8>; setter = Vec<u8>; outer ref = &'a [u8],
    mqtt_format::v5::variable_header::RequestProblemInformation => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::WillDelayInterval => inner = u32; setter = u32; outer deref = u32,
    mqtt_format::v5::variable_header::RequestResponseInformation => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::ResponseInformation<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::ServerReference<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::ReasonString<'i> => inner = String; setter = String; outer ref = &'a str,
    mqtt_format::v5::variable_header::ReceiveMaximum => inner = u16; setter = u16; outer deref = u16,
    mqtt_format::v5::variable_header::TopicAliasMaximum => inner = u16; setter = u16; outer deref = u16,
    mqtt_format::v5::variable_header::TopicAlias => inner = u16; setter = u16; outer deref = u16,
    mqtt_format::v5::variable_header::MaximumQoS => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::RetainAvailable => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::MaximumPacketSize => inner = u32; setter = u32; outer deref = u32,
    mqtt_format::v5::variable_header::WildcardSubscriptionAvailable => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::SubscriptionIdentifiersAvailable => inner = u8; setter = u8; outer deref = u8,
    mqtt_format::v5::variable_header::SharedSubscriptionAvailable => inner = u8; setter = u8; outer deref = u8,
]}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::UserProperty;
    use crate::packets::connect::ConnectProperties;
    use crate::packets::VecWriter;
    use crate::string::MqttString;

    #[test]
    fn check_properties() {
        let mut props = ConnectProperties::new();

        props.with_session_expiry_interval(16u32);
        props.with_user_properties(UserProperty {
            key: MqttString::from_str("foo").unwrap(),
            value: MqttString::from_str("bar").unwrap(),
        });
        for _ in 0..5 {
            props.with_user_properties(UserProperty {
                key: MqttString::from_str("foo").unwrap(),
                value: MqttString::from_str("bar").unwrap(),
            });
        }
        props.with_receive_maximum(4);

        let conn_props = props.as_ref();

        assert_eq!(
            conn_props
                .user_properties()
                .as_ref()
                .unwrap()
                .iter()
                .count(),
            6
        );

        let mut buffer = vec![];
        let mut buffer = VecWriter(&mut buffer);

        conn_props.write(&mut buffer).unwrap();

        let new_props = mqtt_format::v5::packets::connect::ConnectProperties::parse(
            &mut winnow::Bytes::new(&buffer.0),
        )
        .unwrap();

        assert_eq!(conn_props, new_props);
    }
}
