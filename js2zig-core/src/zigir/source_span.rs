// zigir/source_span.rs
// Source location information for diagnostics and source maps.

/// Source location in the original JS file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    /// 1-based line number in the JS source.
    pub js_line: usize,
    /// 1-based column number in the JS source.
    pub js_col: usize,
    /// Source file path (empty string if unavailable).
    pub js_file: String,
}

impl SourceSpan {
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            js_line: line,
            js_col: col,
            js_file: String::new(),
        }
    }

    pub fn with_file(line: usize, col: usize, file: String) -> Self {
        Self {
            js_line: line,
            js_col: col,
            js_file: file,
        }
    }

    /// Format as "line:col" for error messages.
    pub fn to_loc_string(&self) -> String {
        format!("{}:{}", self.js_line, self.js_col)
    }
}

impl std::fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.js_file.is_empty() {
            write!(f, "{}:{}", self.js_line, self.js_col)
        } else {
            write!(f, "{}:{}:{}", self.js_file, self.js_line, self.js_col)
        }
    }
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

/// A diagnostic message attached to an IR module.
#[derive(Debug, Clone)]
pub struct IrDiagnostic {
    pub level: DiagnosticLevel,
    pub span: Option<SourceSpan>,
    pub message: String,
}

impl IrDiagnostic {
    pub fn warning(message: String) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            span: None,
            message,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            span: None,
            message,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

impl std::fmt::Display for IrDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.level {
            DiagnosticLevel::Warning => "warning",
            DiagnosticLevel::Error => "error",
        };
        match &self.span {
            Some(span) => write!(f, "[{}] {}: {}", level, span, self.message),
            None => write!(f, "[{}] {}", level, self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_span_display() {
        let span = SourceSpan::new(10, 5);
        assert_eq!(format!("{}", span), "10:5");
    }

    #[test]
    fn test_source_span_with_file() {
        let span = SourceSpan::with_file(10, 5, "main.js".to_string());
        assert_eq!(format!("{}", span), "main.js:10:5");
    }

    #[test]
    fn test_diagnostic_warning() {
        let d = IrDiagnostic::warning("unused variable".to_string());
        assert_eq!(format!("{}", d), "[warning] unused variable");
    }

    #[test]
    fn test_diagnostic_error_with_span() {
        let d = IrDiagnostic::error("type mismatch".to_string()).with_span(SourceSpan::new(3, 7));
        assert_eq!(format!("{}", d), "[error] 3:7: type mismatch");
    }
}
