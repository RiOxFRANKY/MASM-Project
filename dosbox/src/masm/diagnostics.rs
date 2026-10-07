#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(line: usize, message: impl Into<String>) -> Self {
        Diagnostic { line, severity: Severity::Error, message: message.into() }
    }

    pub fn warning(line: usize, message: impl Into<String>) -> Self {
        Diagnostic { line, severity: Severity::Warning, message: message.into() }
    }

    pub fn render(&self, file: &str) -> String {
        let kind = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        format!("{file}({}): {kind}: {}", self.line, self.message)
    }
}
