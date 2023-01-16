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

    pub fn call(&self, args: &[Box<dyn ClientExecutableCommand>]) -> miette::Result<Command> {
        let mut command = Command::new(&self.path);

        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let args: Vec<_> = args
            .iter()
            .flat_map(|cec| {
                let mut v = vec![cec.as_str().to_string()];
                v.extend(cec.args());
                v
            })
            .collect();

        tracing::debug!(
            "Building command: {} {}",
            self.path.display(),
            args.join(" ")
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quit_command_as_string() {
        assert_eq!("quit", QuitCommand.as_str());
        assert!(QuitCommand.args().is_empty());
    }

    #[test]
    fn test_subscribe_command() {
        let command = Subscribe {
            topic: "foo".to_string(),
        };
        assert_eq!("subscribe", command.as_str());
        assert_eq!(vec!["--topic=foo".to_string()], command.args());
    }

    #[test]
    fn test_send_to_topic_command() {
        let command = SendToTopic {
            topic: "foo".to_string(),
            qos: 0,
            message: "Message".to_string(),
        };
        assert_eq!("send-to-topic", command.as_str());
        assert!(command.args().contains(&"--topic=foo".to_string()));
        assert!(command.args().contains(&"--qos=0".to_string()));
        assert!(command.args().contains(&"--message=Message".to_string()));
    }

    #[test]
    fn test_expect_on_topic() {
        let command = ExpectOnTopic {
            topic: "foo".to_string(),
            qos: 0,
        };
        assert_eq!("expect-on-topic", command.as_str());
        assert!(command.args().contains(&"--topic=foo".to_string()));
        assert!(command.args().contains(&"--qos=0".to_string()));
    }
}
