use crate::ast::Span;
use crate::diagnostics::{Diagnostic, Diagnostics, SourceLocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    String,
    Number,
    HashValue,
    Colon,
    Semicolon,
    Comma,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            span,
        }
    }

    pub fn line(&self) -> usize {
        self.span.line
    }

    pub fn column(&self) -> usize {
        self.span.column
    }
}

#[derive(Debug, Clone)]
pub struct LexedOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Diagnostics,
}

pub struct Lexer<'a> {
    file_name: String,
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    diagnostics: Diagnostics,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file_name: impl Into<String>) -> Self {
        Self {
            file_name: file_name.into(),
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            diagnostics: Diagnostics::new(),
            _source: source,
        }
    }

    pub fn lex(mut self) -> LexedOutput {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\r' | '\n' => self.skip_whitespace(),
                '/' if self.peek_next() == Some('/') => self.skip_line_comment(),
                '/' if self.peek_next() == Some('*') => self.skip_block_comment(),
                '{' => tokens.push(self.single_char(TokenKind::LBrace)),
                '}' => tokens.push(self.single_char(TokenKind::RBrace)),
                '[' => tokens.push(self.single_char(TokenKind::LBracket)),
                ']' => tokens.push(self.single_char(TokenKind::RBracket)),
                ':' => tokens.push(self.single_char(TokenKind::Colon)),
                ';' => tokens.push(self.single_char(TokenKind::Semicolon)),
                ',' => tokens.push(self.single_char(TokenKind::Comma)),
                '"' => tokens.push(self.lex_string()),
                '#' => tokens.push(self.lex_hash_value()),
                '-' => {
                    if self.is_number_start() {
                        tokens.push(self.lex_number_like());
                    } else {
                        tokens.push(self.lex_identifier());
                    }
                }
                '0'..='9' => tokens.push(self.lex_number_like()),
                _ if Self::is_identifier_start(ch) => tokens.push(self.lex_identifier()),
                _ => {
                    let span = self.current_span();
                    let invalid = self.advance().unwrap_or_default();
                    self.diagnostics.push(Diagnostic::invalid_syntax(
                        self.location(span.line, span.column),
                        format!("unexpected character '{}'.", invalid),
                    ));
                }
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            "",
            Span::new(self.line, self.column),
        ));

        LexedOutput {
            tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn single_char(&mut self, kind: TokenKind) -> Token {
        let span = self.current_span();
        let ch = self.advance().unwrap_or_default();
        Token::new(kind, ch.to_string(), span)
    }

    fn lex_string(&mut self) -> Token {
        let start = self.current_span();
        self.advance();

        let mut value = String::new();

        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    return Token::new(TokenKind::String, value, start);
                }
                '\\' => {
                    self.advance();
                    match self.peek() {
                        Some('"') => {
                            value.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some('n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some(other) => {
                            value.push(other);
                            self.advance();
                        }
                        None => break,
                    }
                }
                _ => {
                    value.push(ch);
                    self.advance();
                }
            }
        }

        self.diagnostics.push(Diagnostic::invalid_syntax(
            self.location(start.line, start.column),
            "unterminated string literal.",
        ));

        Token::new(TokenKind::String, value, start)
    }

    fn lex_hash_value(&mut self) -> Token {
        let start = self.current_span();
        let mut value = String::new();

        if let Some(ch) = self.advance() {
            value.push(ch);
        }

        while let Some(ch) = self.peek() {
            if Self::is_value_char(ch) {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::HashValue, value, start)
    }

    fn lex_number_like(&mut self) -> Token {
        let start = self.current_span();
        let mut value = String::new();

        if self.peek() == Some('-') {
            value.push('-');
            self.advance();
        }

        let mut seen_dot = false;

        while let Some(ch) = self.peek() {
            match ch {
                '0'..='9' => {
                    value.push(ch);
                    self.advance();
                }
                '.' if !seen_dot => {
                    seen_dot = true;
                    value.push(ch);
                    self.advance();
                }
                '%' => {
                    value.push(ch);
                    self.advance();
                    break;
                }
                _ if Self::is_unit_char(ch) => {
                    while let Some(unit) = self.peek() {
                        if Self::is_unit_char(unit) {
                            value.push(unit);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    break;
                }
                _ => break,
            }
        }

        Token::new(TokenKind::Number, value, start)
    }

    fn lex_identifier(&mut self) -> Token {
        let start = self.current_span();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            if Self::is_identifier_char(ch) {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::Identifier, value, start)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
            self.advance();
        }
    }

    fn skip_line_comment(&mut self) {
        self.advance();
        self.advance();

        while let Some(ch) = self.peek() {
            self.advance();
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.current_span();
        self.advance();
        self.advance();

        while let Some(ch) = self.peek() {
            if ch == '*' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }

        self.diagnostics.push(Diagnostic::invalid_syntax(
            self.location(start.line, start.column),
            "unterminated block comment.",
        ));
    }

    fn is_number_start(&self) -> bool {
        match (self.peek(), self.peek_next()) {
            (Some('-'), Some(next)) => next.is_ascii_digit(),
            (Some(ch), _) => ch.is_ascii_digit(),
            _ => false,
        }
    }

    fn current_span(&self) -> Span {
        Span::new(self.line, self.column)
    }

    fn location(&self, line: usize, column: usize) -> SourceLocation {
        SourceLocation::new(self.file_name.clone(), line, column)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.index).copied()?;
        self.index += 1;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    fn is_identifier_start(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_' || ch == '-'
    }

    fn is_identifier_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '@')
    }

    fn is_unit_char(ch: char) -> bool {
        ch.is_ascii_alphabetic()
    }

    fn is_value_char(ch: char) -> bool {
        !matches!(
            ch,
            ' ' | '\t' | '\r' | '\n' | '{' | '}' | '[' | ']' | ':' | ';' | ',' | '"'
        )
    }
}
