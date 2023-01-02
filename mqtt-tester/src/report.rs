//
//   This Source Code Form is subject to the terms of the Mozilla Public
//   License, v. 2.0. If a copy of the MPL was not distributed with this
//   file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use std::io::Write;

#[derive(Debug, PartialEq, Eq)]
pub enum ReportResult {
    Success,
    Failure,
    Inconclusive,
}

pub struct Report {
    pub name: String,
    pub description: String,
    pub normative_statement_number: String,
    pub result: ReportResult,
    pub output: Option<Vec<u8>>,
}

impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Report")
            .field("name", &self.name)
            .field("description", &self.description)
            .field(
                "normative_statement_number",
                &self.normative_statement_number,
            )
            .field("result", &self.result)
            .field(
                "output",
                &self.output.as_ref().map(|out| String::from_utf8_lossy(out)),
            )
            .finish()
    }
}

pub fn print_report(report: &Report, mut writer: impl Write) -> Result<(), std::io::Error> {
    use ansi_term::Colour::{Blue, Green, Red, Yellow};
    write!(writer, "{} ... ", Blue.paint(&report.name))?;

    match report.result {
        ReportResult::Success => write!(writer, "{}", Green.paint("ok"))?,
        ReportResult::Failure => {
            writeln!(writer, "{}", Red.paint("failed"))?;
            writeln!(writer, "  {}", report.description)?;
            if let Some(output) = report.output.as_ref() {
                writeln!(
                    writer,
                    "{}",
                    textwrap::indent(&String::from_utf8_lossy(output), "> ")
                )?;
            } else {
                write!(writer, "  {}", Red.paint("No extra output"))?;
            }
        }
        ReportResult::Inconclusive => write!(writer, "{}", Yellow.paint("inconclusive"))?,
    }

    writeln!(writer)?;
    Ok(())
}
