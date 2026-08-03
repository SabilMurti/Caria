# Caria Agent Example Scripts

Example scripts demonstrating common patterns for AI agents writing Caria code.

## Status

**⚠️ Runtime Blocker**: The Caria embedding API is not yet implemented. All scripts will crash with exit code 132 (SIGILL) because `CariaEngine::eval()` returns `"Embedding API not yet implemented - circular dependency blocker"`.

These scripts are **reference implementations** showing correct Caria syntax and patterns. They will execute once the runtime layer is complete.

## Available Examples

### 1. `simple_arithmetic.cria`
Basic integer arithmetic operations: addition, subtraction, multiplication, division, modulo.

**Demonstrates**: Variable declarations, arithmetic expressions, `say()` for output.

### 2. `fibonacci.cria`
Generates the first 15 Fibonacci numbers using iterative algorithm.

**Demonstrates**: State management, loop control, sequence generation.

### 3. `loop_accumulator.cria`
Sum of 1-10 and factorial calculation using accumulator patterns.

**Demonstrates**: While loops, accumulation, mathematical algorithms.

### 4. `counter_pattern.cria`
Countdown and even number filtering with loops.

**Demonstrates**: Iteration patterns, conditional filtering within loops.

### 5. `conditional_logic.cria`
Grade classification, even/odd check, range validation, multiple conditions.

**Demonstrates**: if/else chains, comparison operators, nested conditionals, boolean logic as integers.

## Language Features Used

All examples use only **implemented Caria features**:
- ✅ Integer literals and variables (`x := 42`)
- ✅ Arithmetic operators (`+`, `-`, `*`, `/`, `%`)
- ✅ Comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- ✅ While loops (`while (condition) { }`)
- ✅ If/else statements (`if (cond) { } else { }`)
- ✅ Say output for integers (`say(value)`)
- ✅ Return statements (`ret 0`)

## Not Yet Available

Functions that exist in FFI stubs but are not callable from scripts:
- ❌ `len()`, `string()`, `split()`, `trim()`, `upper()`, `lower()`
- ❌ `http_get()`, `json_parse()`, `json_stringify()`
- ❌ `file_read_text()`, `file_write_text()`
- ❌ `obj_alloc()`, `obj_set()`, `array_push()`
- ❌ `crypto_random_bytes()`, `base64_encode()`, `regex_match()`

## Running Examples

Once the runtime is implemented:

```bash
cd /path/to/Caria
./target/release/caria run examples/agent_scripts/simple_arithmetic.cria
./target/release/caria run examples/agent_scripts/fibonacci.cria
./target/release/caria run examples/agent_scripts/loop_accumulator.cria
./target/release/caria run examples/agent_scripts/counter_pattern.cria
./target/release/caria run examples/agent_scripts/conditional_logic.cria
```

## For AI Agents

When writing Caria scripts, use these examples as templates for:
1. **Arithmetic computations** - mathematical operations on integers
2. **Iterative algorithms** - loops with state tracking
3. **Conditional logic** - branching based on comparisons
4. **Accumulator patterns** - building results over iterations
5. **Number theory** - prime checking, GCD, digit operations

Avoid using unimplemented library functions until the runtime FFI layer is completed.
