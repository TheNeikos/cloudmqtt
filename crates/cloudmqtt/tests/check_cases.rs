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
        "create_client",
        FunctionVerb::new(|harness: &mut TestHarness, name: String| {
            harness.create_client(name).into_diagnostic()
        }),
    );

    ts.add_verb(
        "connect_to_broker",
        FunctionVerb::new(
            |harness: &mut TestHarness, client_name: String, broker_name: String| {
                harness
                    .connect_client_to_broker(client_name, broker_name)
                    .into_diagnostic()
            },
        ),
    );

    ts.add_verb(
        "publish",
        FunctionVerb::new(
            |harness: &mut TestHarness, client_name: String, payload: String, topic: String| {
                harness
                    .publish(client_name, payload, topic)
                    .into_diagnostic()
            },
        ),
    );

    ts.add_verb(
        "publish_to_client",
        FunctionVerb::new(
            |harness: &mut TestHarness,
             broker_name: String,
             client_name: String,
             payload: String,
             topic: String| {
                harness
                    .publish_to_client(broker_name, client_name, payload, topic)
                    .into_diagnostic()
            },
        ),
    );

    ts.add_condition(
        "connect_received_on_broker",
        // TODO: This should not be a new_now() but there's no other interface yet???
        test_dsl::condition::Condition::<TestHarness>::new_now(
            |harness: &TestHarness,
             broker_name: String,
             client_name: String,
             client_identifier: String| {
                harness
                    .wait_for_connect_on_broker(broker_name, client_name, client_identifier)
                    .map(|_| true)
                    .into_diagnostic()
            },
        ),
    );

    ts.add_condition(
        "publish_received_on_broker",
        // TODO: This should not be a new_now() but there's no other interface yet???
        test_dsl::condition::Condition::<TestHarness>::new_now(
            |harness: &TestHarness,
             broker_name: String,
             client_name: String,
             payload: String,
             topic: String| {
                match harness.wait_for_publish_on_broker(broker_name, client_name, payload, topic) {
                    Err(cloudmqtt::test_harness::error::TestHarnessError::PacketNotExpected {
                        got: _,
                    }) => Ok(false),
                    other => other.map(|_| true).into_diagnostic(),
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
        .parse_document(test_dsl::miette::NamedSource::new(
            path,
            std::sync::Arc::from(data),
        ))
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
