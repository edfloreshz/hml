use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub fn new(file: impl Into<PathBuf>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    pub fn with_line(file: impl Into<PathBuf>, line: usize) -> Self {
        Self::new(file, line, 1)
    }

    pub fn file_display(&self) -> String {
        display_path(&self.file)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub element: Option<String>,
    pub message: String,
    pub note: Option<String>,
    pub location: SourceLocation,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        location: SourceLocation,
        element: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            element: element.map(|value| value.into()),
            message: message.into(),
            note: None,
            location,
        }
    }

    pub fn error(
        location: SourceLocation,
        element: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, location, element, message)
    }

    pub fn warning(
        location: SourceLocation,
        element: Option<impl Into<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, location, element, message)
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn unknown_property(
        location: SourceLocation,
        element: impl Into<String>,
        property: impl AsRef<str>,
    ) -> Self {
        let property = property.as_ref();
        Self::warning(
            location,
            Some(element.into()),
            format!("unknown property '{}'.", property),
        )
        .with_note("Passing through to output. Verify this is intentional.")
    }

    pub fn missing_required_attribute(
        location: SourceLocation,
        element: impl Into<String>,
        attribute: impl AsRef<str>,
    ) -> Self {
        let attribute = attribute.as_ref();
        Self::error(
            location,
            Some(element.into()),
            format!("missing required attribute '{}'.", attribute),
        )
    }

    pub fn unexpected_token(
        location: SourceLocation,
        found: impl AsRef<str>,
        expected: impl AsRef<str>,
    ) -> Self {
        Self::error(
            location,
            Option::<String>::None,
            format!(
                "unexpected token '{}'; expected {}.",
                found.as_ref(),
                expected.as_ref()
            ),
        )
    }

    pub fn invalid_syntax(location: SourceLocation, message: impl Into<String>) -> Self {
        Self::error(location, Option::<String>::None, message)
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }

    pub fn header(&self) -> String {
        let subject = self
            .element
            .as_deref()
            .map(|name| format!(" in {}", name))
            .unwrap_or_default();

        format!(
            "{}{} (line {}): {}",
            self.severity.as_str(),
            subject,
            self.location.line,
            self.message
        )
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str(&self.header());

        if let Some(note) = &self.note {
            output.push('\n');
            output.push_str("  ");
            output.push_str(note);
        }

        output.push('\n');
        output.push_str("  --> ");
        output.push_str(&self.location.file_display());
        output.push(':');
        output.push_str(&self.location.line.to_string());

        if self.location.column > 0 {
            output.push(':');
            output.push_str(&self.location.column.to_string());
        }

        output
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(
        &mut self,
        location: SourceLocation,
        element: Option<impl Into<String>>,
        message: impl Into<String>,
    ) {
        self.push(Diagnostic::error(location, element, message));
    }

    pub fn warning(
        &mut self,
        location: SourceLocation,
        element: Option<impl Into<String>>,
        message: impl Into<String>,
    ) {
        self.push(Diagnostic::warning(location, element, message));
    }

    pub fn extend<I>(&mut self, diagnostics: I)
    where
        I: IntoIterator<Item = Diagnostic>,
    {
        self.items.extend(diagnostics);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.items
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(Diagnostic::is_error)
    }

    pub fn has_warnings(&self) -> bool {
        self.items.iter().any(Diagnostic::is_warning)
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| d.is_error()).count()
    }

    pub fn warning_count(&self) -> usize {
        self.items.iter().filter(|d| d.is_warning()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn sort(&mut self) {
        self.items.sort_by(|a, b| {
            (
                display_path(&a.location.file),
                a.location.line,
                a.location.column,
                severity_rank(a.severity),
            )
                .cmp(&(
                    display_path(&b.location.file),
                    b.location.line,
                    b.location.column,
                    severity_rank(b.severity),
                ))
        });
    }

    pub fn render(&self) -> String {
        self.items
            .iter()
            .map(Diagnostic::render)
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn emit_to_stderr(&self) {
        if !self.items.is_empty() {
            eprintln!("{}", self.render());
        }
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a> IntoIterator for &'a Diagnostics {
    type Item = &'a Diagnostic;
    type IntoIter = std::slice::Iter<'a, Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
