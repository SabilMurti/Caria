use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use caria_lexer::CariaLexer;
use caria_parser::Parser;
use caria_diagnostic::typecheck::TypeChecker;

#[derive(Debug)]
pub struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn validate_document(&self, url: Url, text: String) {
        let mut diagnostics = Vec::new();

        let lexer = CariaLexer::new(&text);
        let mut parser = Parser::new(lexer);
        
        match parser.parse_program() {
            Ok(program) => {
                let mut typechecker = TypeChecker::new();
                if let Err(e) = typechecker.check_program(&program) {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: 0 },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("caria-typecheck".to_string()),
                        message: e.message,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
            }
            Err(e) => {
                let (msg, span) = match e {
                    caria_parser::ParseError::UnexpectedToken(msg) => (msg, 0..0),
                    caria_parser::ParseError::UnexpectedEOF(msg) => (msg, 0..text.len()),
                };

                let start_pos = offset_to_position(&text, span.start);
                let end_pos = offset_to_position(&text, span.end);

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: start_pos,
                        end: end_pos,
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("caria-parser".to_string()),
                    message: msg,
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client.publish_diagnostics(url, diagnostics, None).await;
    }
}

fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;
    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position { line, character }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string(), ">".to_string()]),
                    all_commit_characters: None,
                    completion_item: None,
                    work_done_progress_options: Default::default(),
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Calesteria Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        // Spawn a task that force-exits the process after the shutdown response
        // is delivered to the client. This prevents VS Code from timing out
        // waiting for the server process to die (tower-lsp 0.20 doesn't auto-exit).
        tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            std::process::exit(0);
        });
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let url = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents.lock().insert(url.clone(), text.clone());
        self.validate_document(url, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            let url = params.text_document.uri;
            let text = change.text;
            self.documents.lock().insert(url.clone(), text.clone());
            self.validate_document(url, text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _uri = params.text_document_position_params.text_document.uri;
        let _position = params.text_document_position_params.position;
        // Simple/Stub hover for now
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("Calesteria element".to_string())),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        // Simple goto definition returning self/start of document for stub
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
        })))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let url = params.text_document.uri;
        let docs = self.documents.lock();
        if let Some(text) = docs.get(&url) {
            let formatted = caria_formatter::format_code(text);
            
            // Compute full document range
            let lines: Vec<&str> = text.split('\n').collect();
            let last_line = lines.len().saturating_sub(1);
            let last_char = lines.last().map(|l| l.len()).unwrap_or(0);

            Ok(Some(vec![TextEdit {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: last_line as u32, character: last_char as u32 },
                },
                new_text: formatted,
            }]))
        } else {
            Ok(None)
        }
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let keywords = vec![
            "fn", "let", "mut", "if", "else", "while", "for", "in", "struct", "enum", "mod", "use",
            "route", "view", "serve", "ret", "pub", "query"
        ];
        
        let completions = keywords
            .into_iter()
            .map(|kw| CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Calesteria Keyword".to_string()),
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(completions)))
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
    // If serve() somehow returns on its own, also exit cleanly.
    std::process::exit(0);
}
