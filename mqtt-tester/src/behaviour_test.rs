//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use crate::{
    command::{Input, Output},
    executable::ClientExecutableCommand,
};

#[async_trait::async_trait]
pub trait BehaviourTest {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>>;

    async fn execute(&self, mut input: Input, mut output: Output) -> Result<(), miette::Error>;
}

pub struct WaitForConnect;

#[async_trait::async_trait]
impl BehaviourTest for WaitForConnect {
    fn commands(&self) -> Vec<Box<dyn ClientExecutableCommand>> {
        vec![Box::new(crate::executable::QuitCommand)]
    }

    async fn execute(&self, _input: Input, mut output: Output) -> Result<(), miette::Error> {
        output
            .wait_and_check(
                &(|bytes: &[u8]| -> bool {
                    let connect_flags = if let Some(flags) = find_connect_flags(bytes) {
                        flags
                    } else {
                        return false;
                    };

                    let username_flag_set = 0 != (connect_flags & 0b1000_0000); // Username flag
                    let password_flag_set = 0 != (connect_flags & 0b0100_0000); // Username flag

                    if username_flag_set {
                        !password_flag_set
                    } else {
                        true
                    }
                }),
            )
            .await
    }
}

fn find_connect_flags(bytes: &[u8]) -> Option<u8> {
    macro_rules! getbyte {
        ($n:tt) => {
            if let Some(b) = bytes.get($n) {
                *b
            } else {
                return None;
            }
        };
    }

    if getbyte!(0) != 0b0001_0000 {
        return None;
    }

    let str_len = getbyte!(4);
    let connect_flag_position = 4usize + (str_len as usize) + 2;
    Some(getbyte!(connect_flag_position))
}
