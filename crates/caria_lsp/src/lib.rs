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

fn get_word_at_position(text: &str, pos: Position) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    if pos.line as usize >= lines.len() {
        return None;
    }
    let line = lines[pos.line as usize];
    let col = pos.character as usize;
    if col > line.len() {
        return None;
    }

    let start = line[..col]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let end = line[col..]
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|idx| col + idx)
        .unwrap_or(line.len());

    if start < end {
        Some(line[start..end].to_string())
    } else {
        None
    }
}

fn get_prefix_before_position(text: &str, pos: Position) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if pos.line as usize >= lines.len() {
        return String::new();
    }
    let line = lines[pos.line as usize];
    let col = (pos.character as usize).min(line.len());
    let trimmed = &line[..col];
    let start = trimmed
        .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
        .map(|idx| idx + 1)
        .unwrap_or(0);
    trimmed[start..].to_string()
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
                document_symbol_provider: Some(OneOf::Left(true)),
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
                name: "Caria Language Server".to_string(),
                version: Some("0.6.0".to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
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
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        
        let docs = self.documents.lock();
        let text = match docs.get(&uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let word = match get_word_at_position(text, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        let hover_text = match word.as_str() {
            "rune" => "### `rune`\nDeclares a first-class function or procedure in Caria.\n\n```cria\nrune add(a: int, b: int) -> int {\n    ret a + b\n}\n```",
            "ret" => "### `ret`\nReturns a value from a rune execution.\n\n```cria\nret result\n```",
            "say" => "### `say`\nStructured standard output print instruction.\n\n```cria\nsay(\"Operational status: active\")\n```",
            "summon" => "### `summon`\nImports a standard library module or namespace.\n\n```cria\nsummon std.crypto\nsummon std.net\n```",
            "guard" => "### `guard ... else`\nEarly exit guard clause. Executes block if condition is false.\n\n```cria\nguard user.is_active else {\n    ret -1\n}\n```",
            "when" => "### `when`\nPattern matching construct.\n\n```cria\nwhen status {\n    200 => say(\"OK\")\n    404 => say(\"Not Found\")\n    _ => say(\"Other\")\n}\n```",
            "unless" => "### `unless`\nExecutes block when condition is false.\n\n```cria\nunless has_cache {\n    refresh_cache()\n}\n```",
            "std.crypto" | "crypto" => "### `std.crypto` (Enterprise Cryptography)\n- `sha256(data)` / `sha512(data)`\n- `hash_password(pwd)` / `verify_password(pwd, hash)` (Argon2id)\n- `aes_encrypt(key, plaintext)` / `aes_decrypt(key, ciphertext)` (AES-256-GCM)\n- `ed25519_create_keypair()`\n- `ed25519_sign_message(secret, msg)`\n- `ed25519_verify_signature(public, msg, sig)`\n- `uuid_v4()` / `random_hex(count)` / `hex_encode` / `hex_decode`",
            "std.net" | "net" => "### `std.net` (Enterprise HTTP & Router)\n- `get(url)` / `post(url, body)` / `put(url, body)` / `delete(url)`\n- `request(method, url, headers, body)` (Advanced client with bearer tokens)\n- `route(method, path, handler)` (Dynamic router)\n- `server_start(port)` / `server_stop(id)` (Background daemon)",
            "std.db" | "db" => "### `std.db` (Database Engine & SQLite ORM)\n- `open(path)` / `close(conn)`\n- `exec(conn, sql)` / `query(conn, sql)`\n- `exec_params(conn, sql, params)` / `query_params(conn, sql, params)`\n- `run_migration(conn, sql)` / `table_exists(conn, table)`\n- `select_where(conn, table, condition)`\n- `create_pool(path, size)` / `acquire(pool)` / `release(pool, conn)`",
            "std.compress" | "compress" => "### `std.compress` (Compression & Archives)\n- `gzip(data)` / `gunzip(b64)`\n- `tar_pack(src_dir, dest_tar)` / `tar_unpack(tar_path, dest_dir)`\n- `zip_pack(src_file, dest_zip)` / `zip_unpack(zip_path, dest_dir)`",
            "std.ws" | "ws" => "### `std.ws` (Real-Time WebSockets)\n- `connect(url)` / `send(id, msg)` / `recv(id)` / `close(id)`\n- `server_start(port)` / `server_stop(id)`",
            "std.string" | "string" => "### `std.string` (String Manipulation)\n- `concat(a, b)` / `len(s)` / `trim(s)` / `trim_left(s)` / `trim_right(s)`\n- `lower(s)` / `upper(s)` / `contains(s, needle)`\n- `starts_with(s, prefix)` / `ends_with(s, suffix)`\n- `index_of(s, sub)` / `last_index_of(s, sub)`\n- `pad_left(s, len, char)` / `pad_right(s, len, char)` / `repeat(s, count)`",
            "std.regex" | "regex" => "### `std.regex` (Regular Expressions)\n- `validate(pattern)` / `is_match(pattern, text)`\n- `find(pattern, text)` / `replace_pattern(pattern, text, repl)`\n- `replace_all(pattern, text, repl)` / `split(pattern, text)` / `escape(text)`",
            "std.environ" | "environ" => "### `std.environ` (Environment Variables)\n- `get(key)` / `get_or(key, default)`\n- `set(key, val)` / `has(key)`",
            "std.time" | "time" => "### `std.time` (Chrono & Timestamps)\n- `now()` / `now_millis()` / `iso_now()`\n- `format(fmt, timestamp)` / `parse_iso(iso)`",
            "std.math" | "math" => "### `std.math` (Number Theory & Math)\n- `abs(n)` / `min(a, b)` / `max(a, b)` / `clamp(v, min, max)` / `pow(b, e)` / `sqrt(n)`\n- `gcd(a, b)` / `lcm(a, b)` / `factorial(n)` / `is_even(n)` / `is_odd(n)` / `sign(n)`",
            "std.collections" | "collections" => "### `std.collections` (Collections)\n- `range(start, stop)` / `sum(arr)` / `contains_int(arr, val)`\n- `first(arr)` / `last(arr)` / `max_val(arr)` / `min_val(arr)` / `reverse(arr)` / `count(arr, val)`",
            "std.test" | "test" => "### `std.test` (Testing Assertions)\n- `assert_eq(actual, expected, msg)`\n- `assert_str_eq(actual, expected, msg)`\n- `assert_true(condition, msg)`\n- `assert_false(condition, msg)`\n- `assert_contains(haystack, needle, msg)`",
            _ => return Ok(None),
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_text.to_string(),
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        
        let docs = self.documents.lock();
        let text = match docs.get(&uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let target_word = match get_word_at_position(text, pos) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Scan document lines for `rune <target_word>`
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("rune ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let fn_name = parts[1].split('(').next().unwrap_or("");
                    if fn_name == target_word {
                        let char_idx = line.find(fn_name).unwrap_or(0);
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                            uri,
                            range: Range {
                                start: Position { line: line_idx as u32, character: char_idx as u32 },
                                end: Position { line: line_idx as u32, character: (char_idx + fn_name.len()) as u32 },
                            },
                        })));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let docs = self.documents.lock();
        let text = match docs.get(&uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        let mut symbols = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("rune ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let fn_name = parts[1].split('(').next().unwrap_or("");
                    let char_idx = line.find(fn_name).unwrap_or(0);
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name: fn_name.to_string(),
                        detail: Some("Caria Rune".to_string()),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position { line: line_idx as u32, character: 0 },
                            end: Position { line: line_idx as u32, character: line.len() as u32 },
                        },
                        selection_range: Range {
                            start: Position { line: line_idx as u32, character: char_idx as u32 },
                            end: Position { line: line_idx as u32, character: (char_idx + fn_name.len()) as u32 },
                        },
                        children: None,
                    });
                }
            }
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let url = params.text_document.uri;
        let docs = self.documents.lock();
        if let Some(text) = docs.get(&url) {
            let formatted = caria_formatter::format_code(text);
            
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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let docs = self.documents.lock();
        let prefix = if let Some(text) = docs.get(&uri) {
            get_prefix_before_position(text, pos)
        } else {
            String::new()
        };

        // Module-specific completions when typing `module.`
        if prefix.ends_with('.') {
            let mod_name = prefix.trim_end_matches('.');
            let items: Vec<(&str, &str)> = match mod_name {
                "crypto" => vec![
                    ("sha256", "sha256(data: txt) -> txt"),
                    ("sha512", "sha512(data: txt) -> txt"),
                    ("hash_password", "hash_password(pwd: txt) -> txt"),
                    ("verify_password", "verify_password(pwd: txt, hash: txt) -> flag"),
                    ("aes_encrypt", "aes_encrypt(key: txt, plaintext: txt) -> txt"),
                    ("aes_decrypt", "aes_decrypt(key: txt, cipher: txt) -> txt"),
                    ("ed25519_create_keypair", "ed25519_create_keypair() -> txt"),
                    ("ed25519_sign_message", "ed25519_sign_message(secret: txt, msg: txt) -> txt"),
                    ("ed25519_verify_signature", "ed25519_verify_signature(pub: txt, msg: txt, sig: txt) -> flag"),
                    ("uuid_v4", "uuid_v4() -> txt"),
                    ("random_hex", "random_hex(count: int) -> txt"),
                    ("hex_encode", "hex_encode(data: txt) -> txt"),
                    ("hex_decode", "hex_decode(hex_str: txt) -> txt"),
                ],
                "net" => vec![
                    ("get", "get(url: txt) -> txt"),
                    ("post", "post(url: txt, body: txt) -> txt"),
                    ("put", "put(url: txt, body: txt) -> txt"),
                    ("delete", "delete(url: txt) -> txt"),
                    ("request", "request(method: txt, url: txt, headers: txt, body: txt) -> txt"),
                    ("route", "route(method: txt, path: txt, handler_fn: int) -> int"),
                    ("dispatch", "dispatch(method: txt, path: txt, body: txt) -> txt"),
                    ("server_start", "server_start(port: int) -> int"),
                    ("server_stop", "server_stop(server_id: int) -> int"),
                ],
                "db" => vec![
                    ("open", "open(path: txt) -> int"),
                    ("close", "close(conn: int) -> int"),
                    ("exec", "exec(conn: int, sql: txt) -> int"),
                    ("query", "query(conn: int, sql: txt) -> txt"),
                    ("exec_params", "exec_params(conn: int, sql: txt, params_json: txt) -> int"),
                    ("query_params", "query_params(conn: int, sql: txt, params_json: txt) -> txt"),
                    ("run_migration", "run_migration(conn: int, migration_sql: txt) -> int"),
                    ("table_exists", "table_exists(conn: int, table_name: txt) -> flag"),
                    ("select_where", "select_where(conn: int, table: txt, condition: txt) -> txt"),
                ],
                "fs" => vec![
                    ("read", "read(path: txt) -> txt"),
                    ("write", "write(path: txt, content: txt) -> int"),
                    ("append", "append(path: txt, content: txt) -> int"),
                    ("remove", "remove(path: txt) -> int"),
                    ("exists", "exists(path: txt) -> flag"),
                    ("size", "size(path: txt) -> int"),
                    ("mkdir", "mkdir(path: txt) -> int"),
                    ("rmdir", "rmdir(path: txt) -> int"),
                    ("list", "list(path: txt) -> txt"),
                    ("copy", "copy(src: txt, dest: txt) -> int"),
                ],
                "compress" => vec![
                    ("gzip", "gzip(data: txt) -> txt"),
                    ("gunzip", "gunzip(b64: txt) -> txt"),
                    ("tar_pack", "tar_pack(src_dir: txt, dest_tar: txt) -> int"),
                    ("tar_unpack", "tar_unpack(tar_path: txt, dest_dir: txt) -> int"),
                    ("zip_pack", "zip_pack(src_file: txt, dest_zip: txt) -> int"),
                    ("zip_unpack", "zip_unpack(zip_path: txt, dest_dir: txt) -> int"),
                ],
                "ws" => vec![
                    ("connect", "connect(url: txt) -> int"),
                    ("send", "send(ws_id: int, msg: txt) -> int"),
                    ("recv", "recv(ws_id: int) -> txt"),
                    ("close", "close(ws_id: int) -> int"),
                    ("server_start", "server_start(port: int) -> int"),
                    ("server_stop", "server_stop(server_id: int) -> int"),
                ],
                "string" => vec![
                    ("concat", "concat(s1: txt, s2: txt) -> txt"),
                    ("len", "len(s: txt) -> int"),
                    ("trim", "trim(s: txt) -> txt"),
                    ("trim_left", "trim_left(s: txt) -> txt"),
                    ("trim_right", "trim_right(s: txt) -> txt"),
                    ("lower", "lower(s: txt) -> txt"),
                    ("upper", "upper(s: txt) -> txt"),
                    ("contains", "contains(haystack: txt, needle: txt) -> flag"),
                    ("starts_with", "starts_with(s: txt, prefix: txt) -> flag"),
                    ("ends_with", "ends_with(s: txt, suffix: txt) -> flag"),
                    ("pad_left", "pad_left(s: txt, target_len: int, pad_char: txt) -> txt"),
                    ("pad_right", "pad_right(s: txt, target_len: int, pad_char: txt) -> txt"),
                    ("repeat", "repeat(s: txt, count: int) -> txt"),
                ],
                "regex" => vec![
                    ("validate", "validate(pattern: txt) -> flag"),
                    ("is_match", "is_match(pattern: txt, text: txt) -> flag"),
                    ("find", "find(pattern: txt, text: txt) -> int"),
                    ("replace_pattern", "replace_pattern(pattern: txt, text: txt, repl: txt) -> txt"),
                    ("replace_all", "replace_all(pattern: txt, text: txt, repl: txt) -> txt"),
                    ("split", "split(pattern: txt, text: txt) -> txt"),
                    ("escape", "escape(text: txt) -> txt"),
                ],
                "environ" => vec![
                    ("get", "get(key: txt) -> txt"),
                    ("get_or", "get_or(key: txt, default_val: txt) -> txt"),
                    ("set", "set(key: txt, val: txt) -> int"),
                    ("has", "has(key: txt) -> flag"),
                ],
                "math" => vec![
                    ("abs", "abs(n: int) -> int"),
                    ("min", "min(a: int, b: int) -> int"),
                    ("max", "max(a: int, b: int) -> int"),
                    ("clamp", "clamp(val: int, low: int, high: int) -> int"),
                    ("pow", "pow(base: int, exp: int) -> int"),
                    ("sqrt", "sqrt(n: int) -> int"),
                    ("gcd", "gcd(a: int, b: int) -> int"),
                    ("lcm", "lcm(a: int, b: int) -> int"),
                    ("factorial", "factorial(n: int) -> int"),
                    ("is_even", "is_even(n: int) -> flag"),
                    ("is_odd", "is_odd(n: int) -> flag"),
                    ("sign", "sign(n: int) -> int"),
                ],
                "collections" => vec![
                    ("range", "range(start: int, stop: int) -> [int]"),
                    ("sum", "sum(arr: [int]) -> int"),
                    ("contains_int", "contains_int(arr: [int], target: int) -> flag"),
                    ("first", "first(arr: [int]) -> int"),
                    ("last", "last(arr: [int]) -> int"),
                    ("max_val", "max_val(arr: [int]) -> int"),
                    ("min_val", "min_val(arr: [int]) -> int"),
                    ("reverse", "reverse(arr: [int]) -> [int]"),
                    ("count", "count(arr: [int], target: int) -> int"),
                ],
                "test" => vec![
                    ("assert_eq", "assert_eq(actual: int, expected: int, msg: txt) -> int"),
                    ("assert_str_eq", "assert_str_eq(actual: txt, expected: txt, msg: txt) -> int"),
                    ("assert_true", "assert_true(cond: flag, msg: txt) -> int"),
                    ("assert_false", "assert_false(cond: flag, msg: txt) -> int"),
                    ("assert_contains", "assert_contains(haystack: txt, needle: txt, msg: txt) -> int"),
                ],
                _ => Vec::new(),
            };

            let completions = items
                .into_iter()
                .map(|(label, detail)| CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(detail.to_string()),
                    ..Default::default()
                })
                .collect();

            return Ok(Some(CompletionResponse::Array(completions)));
        }

        // Global completions: Keywords + Types + std Modules
        let mut completions = Vec::new();

        let keywords = vec![
            ("rune", "Declares a function or procedure"),
            ("ret", "Returns from a rune"),
            ("say", "Standard formatted output"),
            ("summon", "Imports a standard library module"),
            ("guard", "Early exit guard clause"),
            ("when", "Pattern matching construct"),
            ("unless", "Executes when condition is false"),
            ("if", "Conditional branch"),
            ("else", "Alternative branch"),
            ("while", "Loop while condition holds"),
            ("for", "Loop over collections"),
            ("in", "Collection iteration keyword"),
            ("defer", "Deferred cleanup on exit"),
            ("retry", "Retries block on failure"),
            ("parallel", "Parallel execution block"),
        ];

        for (kw, doc) in keywords {
            completions.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        let types = vec![
            ("int", "64-bit signed integer"),
            ("dec", "64-bit IEEE float"),
            ("txt", "UTF-8 string"),
            ("flag", "Boolean flag (true/false)"),
            ("nil", "Null/Nil value"),
        ];

        for (ty, doc) in types {
            completions.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        let std_modules = vec![
            ("std.crypto", "Enterprise Cryptography (AES, Argon2, Ed25519, SHA)"),
            ("std.net", "HTTP Client, Dynamic Router, Server Daemon"),
            ("std.db", "SQLite Database ORM, Migrations, Pool"),
            ("std.ws", "Real-Time WebSockets Client & Echo Server"),
            ("std.compress", "Compression & Archives (GZIP, TAR, ZIP)"),
            ("std.fs", "Filesystem Operations (Read, Write, Append, List)"),
            ("std.string", "String Manipulation & Search"),
            ("std.regex", "Regular Expressions Engine"),
            ("std.json", "JSON Parser, Serializer & Field Extractor"),
            ("std.environ", "Environment Variable Management"),
            ("std.math", "Advanced Math & Number Theory"),
            ("std.collections", "Collection Iteration & Aggregates"),
            ("std.time", "Chrono, ISO Timestamps, Monotonic Time"),
            ("std.process", "Safe Subprocess Command Execution"),
            ("std.sync", "Crossbeam Thread Channels"),
            ("std.task", "Native Task Thread Scheduler"),
            ("std.telemetry", "Prometheus Metrics Registry"),
            ("std.auth", "JWT Signing, Verification & HMAC"),
            ("std.redis", "Redis Key-Value Cache Engine"),
            ("std.test", "Testing Assertions Harness"),
            ("std.log", "Structured Log Levels (INFO, WARN, ERROR)"),
        ];

        for (mod_name, doc) in std_modules {
            completions.push(CompletionItem {
                label: mod_name.to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(completions)))
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
    std::process::exit(0);
}
