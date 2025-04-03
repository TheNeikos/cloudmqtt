//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::str::FromStr;

use camino::Utf8Path;
use cloudmqtt::test_harness::TestHarness;
use test_dsl::miette::IntoDiagnostic;
use test_dsl::verb::FunctionVerb;

datatest_stable::harness! {
    { test = check_cases, root = "tests/cases/", pattern = ".*kdl$" },
}

fn setup_test_dsl() -> test_dsl::TestDsl<TestHarness> {
    let mut ts = test_dsl::TestDsl::<cloudmqtt::test_harness::TestHarness>::new();

    ts.add_verb(
        "start_broker",
        FunctionVerb::new(|harness: &mut TestHarness, name: String| {
            harness.start_broker(name).into_diagnostic()
        }),
    );

    ts.add_verb(
        "sleep",
        FunctionVerb::new(|harness: &mut TestHarness, duration: String| {
            let duration = humantime::parse_duration(&duration).into_diagnostic()?;
            harness.sleep(duration).into_diagnostic()
        }),
    );

    ts.add_verb(
        "create_client",
        FunctionVerb::new(|harness: &mut TestHarness, name: String| {
            harness.create_client(name).into_diagnostic()
        }),
    );

    ts.add_verb(
        "connect_to_broker",
        test_dsl::named_parameters_verb!(
            |harness: &mut TestHarness, client: String, broker: String| {
                harness
                    .connect_client_to_broker(client, broker)
                    .into_diagnostic()
            }
        ),
    );

    ts.add_verb(
        "publish",
        test_dsl::named_parameters_verb!(|harness: &mut TestHarness,
                                          client: String,
                                          payload: String,
                                          topic: String| {
            harness.publish(client, payload, topic).into_diagnostic()
        }),
    );

    ts.add_verb(
        "publish_to_client",
        test_dsl::named_parameters_verb!(|harness: &mut TestHarness,
                                          broker: String,
                                          client: String,
                                          payload: String,
                                          topic: String| {
            harness
                .publish_to_client(broker, client, payload, topic)
                .into_diagnostic()
        }),
    );

    ts.add_condition(
        "connect_received_on_broker",
        // TODO: This should not be a new_now() but there's no other interface yet???
        test_dsl::condition::FunctionCondition::<TestHarness, _>::new_now(
            |harness: &TestHarness,
             broker_name: String,
             client_name: String,
             client_identifier: String| {
                harness
                    .check_for_connect_on_broker(broker_name, client_name, client_identifier)
                    .into_diagnostic()
            },
        ),
    );

    ts.add_condition(
        "publish_received_on_broker",
        // TODO: This should not be a new_now() but there's no other interface yet???
        test_dsl::condition::FunctionCondition::<TestHarness, _>::new_now(
            |harness: &TestHarness,
             broker_name: String,
             client_name: String,
             payload: String,
             topic: String| {
                match harness.check_for_publish_on_broker(broker_name, client_name, payload, topic)
                {
                    Err(cloudmqtt::test_harness::error::TestHarnessError::PacketNotExpected {
                        got: _,
                    }) => Ok(false),
                    other => other.into_diagnostic(),
                }
            },
        ),
    );

    ts
}

fn check_cases(path: &Utf8Path, data: String) -> datatest_stable::Result<()> {
    tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(tracing_subscriber::EnvFilter::from_str("trace").unwrap())
        .init();

    let ts = setup_test_dsl();

    let testcases = ts
        .parse_testcase(test_dsl::TestCaseInput::FromFile {
            filepath: std::sync::Arc::from(path.as_str()),
            contents: std::sync::Arc::from(data.as_str()),
        })
        .map_err(|error| format!("Failed to parse testcase: {error:?}"))?;

    if testcases.is_empty() {
        return Err(String::from("No testcases found").into());
    }

    let prev_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let payload = panic_info.payload();

        #[expect(
            clippy::manual_map,
            reason = "We want to be clear that we return a None if nothing matches"
        )]
        let payload = if let Some(s) = payload.downcast_ref::<&str>() {
            Some(&**s)
        } else if let Some(s) = payload.downcast_ref::<String>() {
            Some(s.as_str())
        } else {
            None
        };

        let location = panic_info.location().map(|l| l.to_string());

        tracing::error!(
            panic.payload = payload,
            panic.location = location,
            "A panic occurred",
        );

        prev_panic(panic_info);
    }));

    let report_handler = test_dsl::miette::GraphicalReportHandler::new_themed(
        test_dsl::miette::GraphicalTheme::unicode(),
    );

    let any_error = testcases
        .into_iter()
        .zip(std::iter::repeat_with(
            cloudmqtt::test_harness::TestHarness::new,
        ))
        .map(|(testcase, mut harness)| testcase.run(&mut harness))
        .filter_map(Result::err)
        .inspect(|error| {
            let mut out = String::new();
            report_handler.render_report(&mut out, error).unwrap();
            println!("{out}");
        })
        .count()
        != 0;

    if any_error {
        Err(Box::new(Error))
    } else {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("At least one Error")]
struct Error;
