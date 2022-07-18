
pub enum ReportResult {
    Success,
    Failure,
    Unconclusive
}

pub struct Report {
    pub name: String,
    pub description: String,
    pub normative_statement_number: String,
    pub result: ReportResult,
}
