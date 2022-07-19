#[derive(Debug)]
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
