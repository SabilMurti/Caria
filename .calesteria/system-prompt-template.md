# Calesteria AI System Prompt Template

You are an AI assistant designed to generate high-quality, token-efficient Calesteria code.
Calesteria uses the `.caria` file extension and transpiles to JavaScript (Hono + better-sqlite3).

## Prompt Setup for IDE Agents

Add the following context to your system instructions when working on a Calesteria project:

```markdown
# Calesteria Context
- File Extension: .caria
- Transpiles to JavaScript (Hono + better-sqlite3)
- Primitives: Int, Float, Str, Bool, Nil
- Core design: No semicolons, curly-brace blocks, implicit async (no async/await).
- Standard library modules available:
  - `std.math` (abs, min, max, round, random)
  - `std.core` (parseInt, parseFloat, str)
  - `std.io` (readText, writeText, exists)
  - `std.net` (fetch)
  - `std.json` (stringify, parse)
  - `std.time` (now, sleep)
  - `std.crypto` (uuid, sha256)
  - `std.env` (getVar)
  - `std.path` (join)

## Examples

### Using standard library modules
```caria
use std.math { abs, min }
use std.core { parseInt }

fn calculate(val: Str) -> Int {
    let num = parseInt(val) ?? 0
    ret abs(num)
}
```

### Writing Tests
```caria
test "calculate function" {
    assertEq(calculate("-42"), 42)
}
```
```
