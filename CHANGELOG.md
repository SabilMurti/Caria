# Changelog

All notable changes to the Calesteria programming language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - Unreleased
### Added
- Parser support for Binary Operations (`+`, `-`, `*`, `/`, `%`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `and`, `or`)
- Parser support for `mut` statements and assignment (`=`)
- Parser support for `if`/`else` control flow expressions
- Pratt parsing implementation for correct operator precedence

## [0.1.0] - 2026-07-25
### Added
- Initial Rust workspace scaffolding with `caria_cli`, `caria_lexer`, `caria_parser`, `caria_codegen_js`, `caria_diagnostic`
- Lexer implementation via `logos` supporting ~30 keywords and literals
- Recursive descent parser capable of parsing `let` bindings and `fn` declarations
- JS Codegen transpiler emitting basic JavaScript
- Comprehensive planning documentation (PRD, Architecture, Roadmap, Risks, Decisions, Syntax)
- `calesteria run` CLI command for execution via Node.js
