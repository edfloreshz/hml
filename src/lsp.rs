use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, Position, Range,
    ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, Url,
};
use tower_lsp::{Client, LanguageServer};

#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    version: Option<i32>,
}

impl DocumentState {
    fn new(text: String, version: Option<i32>) -> Self {
        Self { text, version }
    }

    fn update(&mut self, text: String, version: Option<i32>) {
        self.text = text;
        self.version = version;
    }
}

#[derive(Debug, Default)]
struct ServerState {
    documents: HashMap<Url, DocumentState>,
}

impl ServerState {
    fn upsert(&mut self, uri: Url, text: String, version: Option<i32>) {
        self.documents
            .entry(uri)
            .and_modify(|doc| doc.update(text.clone(), version))
            .or_insert_with(|| DocumentState::new(text, version));
    }

    fn remove(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    fn get(&self, uri: &Url) -> Option<&DocumentState> {
        self.documents.get(uri)
    }
}

pub struct HmlLanguageServer {
    client: Client,
    state: Arc<RwLock<ServerState>>,
}

impl HmlLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ServerState::default())),
        }
    }

    async fn publish_compiler_diagnostics(
        &self,
        uri: Url,
        text: String,
        version: Option<i32>,
    ) -> Result<()> {
        let file_name = uri
            .to_file_path()
            .ok()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| uri.path().to_string());

        let result = crate::compile(&text, file_name);
        let diagnostics = result
            .diagnostics
            .iter()
            .map(compiler_diagnostic_to_lsp)
            .collect::<Vec<_>>();

        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;

        Ok(())
    }

    async fn publish_current_document_diagnostics(&self, uri: &Url) -> Result<()> {
        let (text, version) = {
            let state = self.state.read().await;
            match state.get(uri) {
                Some(doc) => (doc.text.clone(), doc.version),
                None => return Ok(()),
            }
        };

        self.publish_compiler_diagnostics(uri.clone(), text, version)
            .await
    }

    async fn clear_diagnostics(&self, uri: Url) {
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    fn merge_content_changes(changes: Vec<TextDocumentContentChangeEvent>) -> String {
        changes
            .into_iter()
            .last()
            .map(|change| change.text)
            .unwrap_or_default()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for HmlLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(
                            tower_lsp::lsp_types::TextDocumentSyncSaveOptions::Supported(true),
                        ),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "HML language server initialized with compiler-backed diagnostics.",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document = params.text_document;
        let uri = document.uri.clone();
        let version = Some(document.version);
        let text = document.text;

        {
            let mut state = self.state.write().await;
            state.upsert(uri.clone(), text.clone(), version);
        }

        let _ = self.publish_compiler_diagnostics(uri, text, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = Some(params.text_document.version);
        let text = Self::merge_content_changes(params.content_changes);

        {
            let mut state = self.state.write().await;
            state.upsert(uri.clone(), text.clone(), version);
        }

        let _ = self.publish_compiler_diagnostics(uri, text, version).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let _ = self.publish_current_document_diagnostics(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        {
            let mut state = self.state.write().await;
            state.remove(&uri);
        }

        self.clear_diagnostics(uri).await;
    }
}

fn compiler_diagnostic_to_lsp(diagnostic: &crate::Diagnostic) -> LspDiagnostic {
    let line = diagnostic.location.line.saturating_sub(1) as u32;
    let column = diagnostic.location.column.saturating_sub(1) as u32;
    let severity = compiler_severity_to_lsp(diagnostic.severity);

    let range = Range {
        start: Position {
            line,
            character: column,
        },
        end: Position {
            line,
            character: column.saturating_add(1),
        },
    };

    let mut message = diagnostic.message.clone();
    if let Some(note) = &diagnostic.note {
        if !note.is_empty() {
            message.push_str("\n\n");
            message.push_str(note);
        }
    }

    LspDiagnostic {
        range,
        severity: Some(severity),
        source: Some("hml".to_string()),
        message,
        ..LspDiagnostic::default()
    }
}

fn compiler_severity_to_lsp(severity: crate::Severity) -> DiagnosticSeverity {
    match severity {
        crate::Severity::Error => DiagnosticSeverity::ERROR,
        crate::Severity::Warning => DiagnosticSeverity::WARNING,
    }
}
