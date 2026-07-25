# Calesteria — Risk Matrix

> **Rating scale:** Probability (Low/Med/High) × Impact (Low/Med/High/Critical)
> **Priority:** P1 (address immediately) → P4 (monitor)

---

## Technical Risks

### TR-01: Parser Complexity Explosion
| Attribute | Value |
|-----------|-------|
| **Probability** | Medium |
| **Impact** | High |
| **Priority** | P1 |
| **Category** | Technical |

**Description:** As the language grows, the grammar may become ambiguous or hard to parse. New features can conflict with existing syntax in subtle ways.

**Examples:**
- Adding generics `<T>` conflicts with less-than operator `<`
- String interpolation `{expr}` conflicts with block syntax `{ stmt }`
- Pattern matching syntax may overlap with struct literals

**Mitigation:**
1. Write formal EBNF grammar BEFORE implementing any syntax
2. Test every new construct against all existing constructs for ambiguity
3. Use Pratt parsing for expressions (handles precedence conflicts gracefully)
4. Keep the grammar small: ~30 keywords, ~15 statement types
5. When in doubt, require explicit delimiters (parentheses, brackets) to disambiguate

**Indicator:** Parser produces unexpected AST or panics on valid-looking code.

---

### TR-02: Type Inference Hitting Limits
| Attribute | Value |
|-----------|-------|
| **Probability** | Medium |
| **Impact** | High |
| **Priority** | P2 |
| **Category** | Technical |

**Description:** Bidirectional type inference can't always determine types. Complex generic chains, higher-kinded types, or recursive types may require explicit annotations that the "inferred" design promised to avoid.

**Examples:**
```
let x = []                    // List of what? Can't infer.
let f = |x| { x + 1 }        // x is Int? Float? Str?
let result = parse(fetch(url)) // chained generics
```

**Mitigation:**
1. Require type annotations in genuinely ambiguous cases (empty collections, unbound closures)
2. Design error messages that suggest the annotation needed
3. Start simple: only local inference (within a function). Don't attempt Hindley-Milner global inference.
4. Follow Kotlin/Swift approach: infer where obvious, require annotation at boundaries

**Indicator:** Compiler emits "cannot infer type" more than 10% of the time in typical code.

---

### TR-03: Implicit Async Detection Errors
| Attribute | Value |
|-----------|-------|
| **Probability** | Medium |
| **Impact** | High |
| **Priority** | P1 |
| **Category** | Technical |

**Description:** The compiler must determine which functions are async by analyzing the call graph. If it misses an async call or over-marks functions as async, generated JS will have runtime errors.

**Scenarios:**
- Dynamic dispatch (calling a function via variable) — can't determine if async
- npm interop functions — compiler doesn't know if they're async
- Recursive async chains
- Closures passed as callbacks

**Mitigation:**
1. npm interop: require explicit `@async` annotation on `use npm` functions that return Promises
2. Dynamic dispatch: default to async (safe) — may over-generate `await` but won't break
3. Build comprehensive test suite specifically for async detection edge cases
4. Allow escape hatch: user can force `@sync` or `@async` annotation when compiler is wrong

**Indicator:** Generated JS throws "X is not a function" or returns `Promise { <pending> }` instead of value.

---

### TR-04: JavaScript Codegen Quality
| Attribute | Value |
|-----------|-------|
| **Probability** | Low |
| **Impact** | Medium |
| **Priority** | P3 |
| **Category** | Technical |

**Description:** Generated JavaScript may be:
- Hard to read/debug (obfuscated variable names, unnecessary wrappers)
- Inefficient (extra allocations, unnecessary closures)
- Incompatible with certain Node.js versions or bundlers

**Mitigation:**
1. Generate human-readable JS with preserved variable names and comments
2. Generate source maps (.caria → .js line mapping)
3. Test generated code against Node 20, 22, Bun, and Deno
4. Benchmark generated code against hand-written equivalents

**Indicator:** Generated JS significantly slower or larger than hand-written equivalent.

---

### TR-05: npm Interop Boundary Issues
| Attribute | Value |
|-----------|-------|
| **Probability** | High |
| **Impact** | Medium |
| **Priority** | P2 |
| **Category** | Technical |

**Description:** Calling npm packages from Calesteria crosses a type boundary. Calesteria has static types; npm packages may return anything. Type mismatches at the boundary cause silent bugs.

**Mitigation:**
1. npm imports are typed as `Any` at the boundary — no false safety guarantees
2. Provide `@typed` annotation for manually declaring npm package types
3. Auto-generate type bindings from TypeScript `.d.ts` files (Phase 3)
4. Runtime type validation option: `use npm "lodash" { debounce } @checked`

**Indicator:** Runtime type errors originating from npm package return values.

---

## Project Risks

### PR-01: Scope Creep
| Attribute | Value |
|-----------|-------|
| **Probability** | Very High |
| **Impact** | Critical |
| **Priority** | P1 |
| **Category** | Project |

**Description:** The vision is massive (general-purpose language + web framework + game engine + desktop). Without discipline, the project will grow endlessly without reaching a usable state.

**Mitigation:**
1. **Strict phase gates:** Each phase has a fixed scope and demo requirement. No feature leaks.
2. **YAGNI enforcement:** If a feature isn't needed for the current phase's demo, it doesn't exist yet.
3. **"Cuts" list:** Every planning session produces an explicit "NOT in this phase" list.
4. **Demo-driven development:** Each phase ends with a concrete, runnable demo — not "the parser now handles 3 more edge cases."

**Indicator:** Phase takes 2x+ estimated time. Feature list grows without features completing.

---

### PR-02: Motivation Drop / Abandonment
| Attribute | Value |
|-----------|-------|
| **Probability** | Medium |
| **Impact** | Critical |
| **Priority** | P1 |
| **Category** | Project |

**Description:** Programming languages take months/years. The gap between "started" and "usable" is long. Motivation can die in the valley.

**Mitigation:**
1. **Quick win Phase 1:** "Hello World" running within the first session. Instant dopamine.
2. **Visual milestones:** Each phase has a screenshot-worthy or demo-worthy moment.
3. **GitHub activity:** Regular small commits maintain streak and visible progress.
4. **Dogfooding early:** Use Calesteria for a real mini-project ASAP (Phase 2).
5. **Public accountability:** Push to GitHub, share progress (even if nobody's watching yet).

**Indicator:** No commits for 2+ weeks. Switching to other projects without completing a phase.

---

### PR-03: Design Regret / Syntax Rewrites
| Attribute | Value |
|-----------|-------|
| **Probability** | High |
| **Impact** | Medium |
| **Priority** | P2 |
| **Category** | Project |

**Description:** After implementing syntax, you realize it's awkward, conflicting, or ugly. You want to change it — but changing syntax means rewriting parser, tests, examples, and docs.

**Mitigation:**
1. **Expect 2-3 major syntax revisions.** Plan for it. This is normal.
2. **Keep test suite comprehensive** — syntax change = rerun tests = confidence in migration.
3. **Don't over-document early.** Markdown specs are cheap to rewrite. Extensive tutorials are not.
4. **Version the syntax:** `calesteria.toml` has `edition = "2026"` for future breaking changes.

**Indicator:** Wanting to change syntax for the 4th+ time on the same construct.

---

### PR-04: Solo Developer Bottleneck
| Attribute | Value |
|-----------|-------|
| **Probability** | Certain |
| **Impact** | Medium |
| **Priority** | P3 |
| **Category** | Project |

**Description:** One person can't build everything. Compiler + type system + codegen + runtime + LSP + docs + website + examples = too much for one human.

**Mitigation:**
1. **Vibecoding leverage:** AI handles implementation. Creator handles design and direction.
2. **Prioritize ruthlessly:** Phase 1-2 = compiler only. Everything else comes later.
3. **Reuse existing work:** Runtime wraps Hono (router), better-sqlite3 (DB), etc.
4. **Open source early:** Even without users, the code is public for future contributors.
5. **Seiza integration:** Use the orchestrator to parallelize development tasks.

**Indicator:** Feeling overwhelmed. Multiple half-finished features.

---

## Market / Adoption Risks

### MR-01: "Why Not Just Use TypeScript?"
| Attribute | Value |
|-----------|-------|
| **Probability** | Certain |
| **Impact** | Low |
| **Priority** | P4 |
| **Category** | Market |

**Description:** Every new language faces this question. TypeScript is good enough for most people.

**Response strategy:**
- This is a learning project first, product second
- The answer is: "Because Calesteria is designed for AI to write, not humans. Try generating a CRUD API in both and count the tokens."
- Don't engage in language wars. Build, demonstrate, let results speak.

---

### MR-02: LLMs Don't Know Calesteria
| Attribute | Value |
|-----------|-------|
| **Probability** | Certain (initially) |
| **Impact** | Medium |
| **Priority** | P2 |
| **Category** | Market |

**Description:** LLMs have zero training data on Calesteria. They will hallucinate syntax, mix in JavaScript/Rust/Python constructs.

**Mitigation:**
1. **In-context learning works.** Research confirms LLMs learn new language syntax from ~10 examples in the prompt. Calesteria ships with a compact spec designed for exactly this.
2. **Small, regular grammar.** Fewer rules = easier to learn from examples. Calesteria has ~30 keywords and ~15 statement forms. That's a 2000-token spec.
3. **MCP server.** IDE agents can call `validate_syntax` to check generated code and `explain_error` to self-correct.
4. **System prompt injection.** `calesteria spec` outputs a pre-formatted system prompt block.
5. **Self-scaffolding loop.** Generate → validate → correct → re-inject corrected examples → generate better. This works without any training data.

**Indicator:** LLMs consistently generate invalid syntax even with spec in context. If this happens, the grammar is too complex — simplify it.

---

### MR-03: No Ecosystem / No Libraries
| Attribute | Value |
|-----------|-------|
| **Probability** | Certain (initially) |
| **Impact** | High |
| **Priority** | P2 |
| **Category** | Market |

**Description:** New language = zero libraries. Users can't build real apps without an ecosystem.

**Mitigation:**
1. **npm interop from Day 1.** `use npm "package"` gives access to 4M+ npm packages instantly.
2. **Built-in web primitives.** Routes, queries, views are in the language — no framework needed.
3. **Standard library.** Core operations (file I/O, JSON, HTTP, math) ship with the language.
4. **Don't build a package registry.** Let npm handle distribution. Calesteria packages can be npm packages too.

---

## Risk Summary Matrix

| ID | Risk | Probability | Impact | Priority | Status |
|----|------|------------|--------|----------|--------|
| TR-01 | Parser complexity | Medium | High | P1 | Open |
| TR-02 | Type inference limits | Medium | High | P2 | Open |
| TR-03 | Implicit async errors | Medium | High | P1 | Open |
| TR-04 | JS codegen quality | Low | Medium | P3 | Open |
| TR-05 | npm interop boundary | High | Medium | P2 | Open |
| PR-01 | Scope creep | Very High | Critical | P1 | Open |
| PR-02 | Motivation drop | Medium | Critical | P1 | Open |
| PR-03 | Design regret | High | Medium | P2 | Open |
| PR-04 | Solo dev bottleneck | Certain | Medium | P3 | Open |
| MR-01 | "Why not TypeScript?" | Certain | Low | P4 | Open |
| MR-02 | LLMs don't know lang | Certain | Medium | P2 | Open |
| MR-03 | No ecosystem | Certain | High | P2 | Open |
