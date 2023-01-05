//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::Command;

pub struct ClientExecutable {
    path: PathBuf,
}

impl ClientExecutable {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn call<I>(&self, args: I) -> miette::Result<Command>
    where
        I: IntoIterator<Item = Box<dyn ClientExecutableCommand>>,
    {
        let mut command = Command::new(&self.path);

        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let args: Vec<_> = args
            .into_iter()
            .flat_map(|cec| {
                let mut v = vec![cec.as_str().to_string()];
                v.extend(cec.args());
                v
            })
            .collect();

        if !args.is_empty() {
            command.args(args);
        }

        Ok(command)
    }
}

pub trait ClientExecutableCommand {
    fn as_str(&self) -> &'static str;
    fn args(&self) -> Vec<String> {
        vec![]
    }
}

macro_rules! define_command {
    ($tyname:ident { $($member:ident : $memty:ty ),+ } => $s:literal, args: $($arg:literal),+) => {
        pub struct $tyname {
            $(pub $member : $memty),*
        }

        impl ClientExecutableCommand for $tyname {
            fn as_str(&self) -> &'static str {
                $s
            }
            fn args(&self) -> Vec<String> {
                $(let $member = &self.$member;)+

                vec![
                    $(format!($arg)),+
                ]
            }
        }
    };
    ($tyname:ident => $s:literal, args: $($arg:literal),+) => {
        pub struct $tyname;

        impl ClientExecutableCommand for $tyname {
            fn as_str(&self) -> &'static str {
                $s
            }
            fn args(&self) -> Vec<String> {
                vec![
                    $($arg.to_string()),+
                ]
            }
        }
    };

    ($tyname:ident => $s:literal) => {
        pub struct $tyname;

        impl ClientExecutableCommand for $tyname {
            fn as_str(&self) -> &'static str {
                $s
            }
        }
    }

}

define_command!(QuitCommand => "quit");
define_command!(Subscribe { topic: String } => "subscribe", args: "--topic={topic}");
define_command!(SendToTopic { topic: String, qos: u8, message: String } => "send-to-topic", args: "--topic={topic}", "--qos={qos}", "--message={message}");
define_command!(ExpectOnTopic { topic: String, qos: u8 } => "expect-on-topic", args: "--topic={topic}", "--qos={qos}");
