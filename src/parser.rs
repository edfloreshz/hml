use crate::ast::{Document, ElementNode, Node, Property, Span, Value};
use crate::diagnostics::{Diagnostic, Diagnostics, SourceLocation};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    file_name: String,
    diagnostics: Diagnostics,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file_name: impl Into<String>) -> Self {
        Self {
            tokens,
            current: 0,
            file_name: file_name.into(),
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn parse(&mut self) -> ParseResult {
        let mut nodes = Vec::new();

        while !self.is_at_end() {
            if self.check(TokenKind::Eof) {
                break;
            }

            match self.parse_node() {
                Some(node) => nodes.push(node),
                None => self.synchronize(),
            }
        }

        ParseResult {
            document: Document::new(nodes),
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    fn parse_node(&mut self) -> Option<Node> {
        self.parse_element().map(Node::new)
    }

    fn parse_element(&mut self) -> Option<ElementNode> {
        let name_token = self.consume_identifier("expected element name")?.clone();
        let name = name_token.lexeme.clone();
        let span = Span::new(name_token.line(), name_token.column());

        self.consume(TokenKind::LBrace, "expected '{' after element name")?;

        let mut properties = Vec::new();
        let mut children = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            if self.is_property_start() {
                if let Some(property) = self.parse_property() {
                    properties.push(property);
                } else {
                    self.synchronize_in_block();
                }
            } else if self.check(TokenKind::Identifier) {
                if let Some(child) = self.parse_node() {
                    children.push(child);
                } else {
                    self.synchronize_in_block();
                }
            } else {
                let token = self.peek().clone();
                self.push_error_at_token(
                    &token,
                    format!(
                        "unexpected token '{}'; expected property or child element.",
                        token.lexeme
                    ),
                );
                self.advance();
            }
        }

        if !self
            .consume(TokenKind::RBrace, "expected '}' after element body")
            .is_some()
        {
            return Some(ElementNode::new(name, properties, children, span));
        }

        Some(ElementNode::new(name, properties, children, span))
    }

    fn parse_property(&mut self) -> Option<Property> {
        let name_token = self.consume_identifier("expected property name")?.clone();
        let name = name_token.lexeme.clone();
        let span = Span::new(name_token.line(), name_token.column());

        self.consume(TokenKind::Colon, "expected ':' after property name")?;

        let value = self.parse_value()?;
        Some(Property::new(name, value, span))
    }

    fn parse_value(&mut self) -> Option<Value> {
        let token = self.peek().clone();

        match token.kind {
            TokenKind::String => {
                self.advance();
                Some(Value::String(token.lexeme))
            }
            TokenKind::Number => {
                self.advance();
                Some(Value::Number(token.lexeme))
            }
            TokenKind::Identifier => {
                self.advance();
                Some(Value::Ident(token.lexeme))
            }
            TokenKind::HashValue => {
                self.advance();
                Some(Value::Raw(token.lexeme))
            }
            _ => {
                self.push_error_at_token(&token, "expected property value".to_string());
                None
            }
        }
    }

    fn is_property_start(&self) -> bool {
        self.check(TokenKind::Identifier) && self.check_next(TokenKind::Colon)
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if self.check(TokenKind::RBrace) {
                return;
            }

            if self.check(TokenKind::Identifier) {
                return;
            }

            self.advance();
        }
    }

    fn synchronize_in_block(&mut self) {
        while !self.is_at_end() {
            if self.check(TokenKind::RBrace) || self.check(TokenKind::Identifier) {
                return;
            }

            self.advance();
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Option<&Token> {
        if self.check(TokenKind::Identifier) {
            return Some(self.advance());
        }

        let token = self.peek().clone();
        self.push_error_at_token(&token, message.to_string());
        None
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Option<&Token> {
        if self.check(kind) {
            return Some(self.advance());
        }

        let token = self.peek().clone();
        self.push_error_at_token(&token, message.to_string());
        None
    }

    fn check_next(&self, kind: TokenKind) -> bool {
        if self.current + 1 >= self.tokens.len() {
            return false;
        }

        self.tokens[self.current + 1].kind == kind
    }

    fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == kind
            || (kind == TokenKind::Eof && self.is_at_end())
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn push_error_at_token(&mut self, token: &Token, message: String) {
        self.diagnostics.push(Diagnostic::error(
            SourceLocation::new(self.file_name.clone(), token.line(), token.column()),
            Option::<String>::None,
            message,
        ));
    }
}

pub struct ParseResult {
    pub document: Document,
    pub diagnostics: Diagnostics,
}
