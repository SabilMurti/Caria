# Caria Usage Guide for AI Agent Developers

This guide provides step-by-step instructions for AI developers writing Caria scripts for automation, agents, and tooling.

## Table of Contents

- [Getting Started](#getting-started)
- [Language Fundamentals](#language-fundamentals)
- [Control Flow Patterns](#control-flow-patterns)
- [Common Patterns](#common-patterns)
- [Best Practices](#best-practices)
- [Debugging](#debugging)
- [Performance Tips](#performance-tips)

## Getting Started

### Installation

```bash
cd /home/murtix/projects/CariaEcosystem/Caria
cargo build --release -p caria_cli
export PATH=$PWD/target/release:$PATH
```

Verify installation:

```bash
caria --help
```

### Your First Script

Create `hello.cria`:

```caria
rune main() -> int {
    say("Hello from Caria!")
    ret 0
}
```

Run it:

```bash
caria run hello.cria
```

Expected output:
```
Hello from Caria!
```

Exit code: `0`

### Script Structure

Every Caria script must have a `main` rune (function) that returns an integer exit code:

```caria
rune main() -> int {
    // Your code here
    ret 0  // Success
}
```

Exit codes:
- `0` = Success
- Non-zero = Error (e.g., `1`, `2`, etc.)

## Language Fundamentals

### Variables

Variables in Caria are declared with `:=` and are mutable by default:

```caria
rune main() -> int {
    // Integer
    x := 42
    
    // Float (future support)
    // price := 19.99
    
    // Boolean
    flag := 1  // true
    flag := 0  // false
    
    // Reassignment
    x := x + 10
    say(x)  // 52
    
    ret 0
}
```

### Operators

#### Arithmetic

```caria
rune main() -> int {
    a := 10
    b := 3
    
    say(a + b)  // 13 - Addition
    say(a - b)  // 7  - Subtraction
    say(a * b)  // 30 - Multiplication
    say(a / b)  // 3  - Division (integer)
    say(a % b)  // 1  - Modulo (remainder)
    
    ret 0
}
```

#### Comparison

```caria
rune main() -> int {
    x := 10
    y := 20
    
    result := x > y   // 0 (false)
    result := x < y   // 1 (true)
    result := x >= 10 // 1 (true)
    result := x <= 5  // 0 (false)
    result := x == 10 // 1 (true)
    result := x != 5  // 1 (true)
    
    ret 0
}
```

#### Logical

```caria
rune main() -> int {
    a := 1  // true
    b := 0  // false
    
    result := a && b  // 0 (false) - AND
    result := a || b  // 1 (true)  - OR
    result := !a      // 0 (false) - NOT
    
    ret 0
}
```

### Output

Use `say()` to print values to stdout:

```caria
rune main() -> int {
    say(42)
    say(100)
    
    x := 15
    say(x)
    
    ret 0
}
```

Output:
```
42
100
15
```

**Note**: Currently, `say()` only supports integers. String support is planned.

## Control Flow Patterns

### If/Else If/Else

```caria
rune main() -> int {
    score := 85
    
    if (score >= 90) {
        say(4)  // A
    } else if (score >= 80) {
        say(3)  // B
    } else if (score >= 70) {
        say(2)  // C
    } else if (score >= 60) {
        say(1)  // D
    } else {
        say(0)  // F
    }
    
    ret 0
}
```

**Important**: 
- Condition must be wrapped in parentheses: `if (condition)`
- Use `==` for equality, not `=`

### While Loop

```caria
rune main() -> int {
    counter := 0
    
    while (counter < 5) {
        say(counter)
        counter := counter + 1
    }
    
    ret 0
}
```

Output:
```
0
1
2
3
4
```

### For Range Loop

```caria
rune main() -> int {
    // Basic range (0 to 9)
    for i in 0..10 {
        say(i)
    }
    
    // Range with step
    for i in 0..20 step 2 {
        say(i)  // 0, 2, 4, 6, 8, ...
    }
    
    ret 0
}
```

**Note**: Range is exclusive of the end value (0..10 means 0-9).

## Common Patterns

### Accumulator Pattern

```caria
rune main() -> int {
    sum := 0
    i := 1
    
    while (i <= 100) {
        sum := sum + i
        i := i + 1
    }
    
    say(sum)  // 5050
    ret 0
}
```

### Counter Pattern

```caria
rune main() -> int {
    count := 0
    i := 0
    
    while (i < 100) {
        if (i % 2 == 0) {
            count := count + 1
        }
        i := i + 1
    }
    
    say(count)  // 50 (even numbers from 0-99)
    ret 0
}
```

### Factorial Calculation

```caria
rune main() -> int {
    n := 10
    factorial := 1
    
    while (n > 0) {
        factorial := factorial * n
        n := n - 1
    }
    
    say(factorial)  // 3628800
    ret 0
}
```

### Fibonacci Generator

```caria
rune main() -> int {
    a := 0
    b := 1
    count := 0
    limit := 15
    
    while (count < limit) {
        say(a)
        temp := a + b
        a := b
        b := temp
        count := count + 1
    }
    
    ret 0
}
```

Output:
```
0
1
1
2
3
5
8
13
21
34
55
89
144
233
377
```

### FizzBuzz

```caria
rune main() -> int {
    i := 1
    
    while (i <= 100) {
        divisible_by_3 := (i % 3) == 0
        divisible_by_5 := (i % 5) == 0
        
        if (divisible_by_3 && divisible_by_5) {
            say(15)  // FizzBuzz indicator
        } else if (divisible_by_3) {
            say(3)   // Fizz indicator
        } else if (divisible_by_5) {
            say(5)   // Buzz indicator
        } else {
            say(i)
        }
        
        i := i + 1
    }
    
    ret 0
}
```

### Prime Number Checker

```caria
rune main() -> int {
    num := 17
    is_prime := 1
    
    if (num <= 1) {
        is_prime := 0
    } else {
        i := 2
        while (i * i <= num) {
            if (num % i == 0) {
                is_prime := 0
            }
            i := i + 1
        }
    }
    
    say(is_prime)  // 1 (true, 17 is prime)
    ret 0
}
```

### Find Maximum

```caria
rune main() -> int {
    // Simulating array with individual variables
    val1 := 42
    val2 := 17
    val3 := 99
    val4 := 8
    val5 := 56
    
    max := val1
    
    if (val2 > max) {
        max := val2
    }
    if (val3 > max) {
        max := val3
    }
    if (val4 > max) {
        max := val4
    }
    if (val5 > max) {
        max := val5
    }
    
    say(max)  // 99
    ret 0
}
```

## Best Practices

### 1. Use Descriptive Variable Names

**Bad:**
```caria
x := 10
y := 20
z := x + y
```

**Good:**
```caria
width := 10
height := 20
area := width * height
```

### 2. Initialize Variables Before Use

```caria
rune main() -> int {
    sum := 0      // Initialize accumulator
    count := 0    // Initialize counter
    
    // Use variables
    while (count < 10) {
        sum := sum + count
        count := count + 1
    }
    
    ret 0
}
```

### 3. Use Early Returns for Error Cases

```caria
rune main() -> int {
    value := 0
    
    // Check for error condition first
    if (value < 0) {
        ret 1  // Early return with error code
    }
    
    // Normal processing
    result := value * 2
    say(result)
    
    ret 0
}
```

### 4. Break Complex Conditions into Variables

**Bad:**
```caria
if ((x > 0) && (x < 100) && ((x % 2) == 0)) {
    // ...
}
```

**Good:**
```caria
is_positive := x > 0
is_less_than_100 := x < 100
is_even := (x % 2) == 0
is_valid := is_positive && is_less_than_100 && is_even

if (is_valid) {
    // ...
}
```

### 5. Use Consistent Indentation

```caria
rune main() -> int {
    x := 10
    
    if (x > 5) {
        y := x * 2
        
        if (y > 15) {
            say(y)
        }
    }
    
    ret 0
}
```

### 6. Avoid Magic Numbers

**Bad:**
```caria
if (status == 2) {
    // What does 2 mean?
}
```

**Good:**
```caria
STATUS_SUCCESS := 0
STATUS_ERROR := 1
STATUS_PENDING := 2

if (status == STATUS_PENDING) {
    // Clear meaning
}
```

### 7. Check Exit Codes

Always return meaningful exit codes:

```caria
rune main() -> int {
    value := 10
    
    if (value < 0) {
        ret 1  // Invalid input
    }
    
    if (value > 100) {
        ret 2  // Out of range
    }
    
    // Success
    ret 0
}
```

## Debugging

### Print Debugging

Use `say()` to trace execution:

```caria
rune main() -> int {
    x := 10
    say(x)  // Debug: check initial value
    
    x := x * 2
    say(x)  // Debug: check after operation
    
    ret 0
}
```

### Check Conditions

```caria
rune main() -> int {
    value := 15
    
    // Debug condition evaluation
    condition := value > 10
    say(condition)  // Should print 1 (true)
    
    if (condition) {
        say(999)  // Marker to show this branch executed
    }
    
    ret 0
}
```

### Incremental Testing

Test small pieces:

```caria
rune main() -> int {
    // Test 1: Basic arithmetic
    x := 5
    y := 10
    sum := x + y
    say(sum)  // Should be 15
    
    // Test 2: Comparison
    result := x < y
    say(result)  // Should be 1 (true)
    
    ret 0
}
```

### Common Errors

#### 1. Using `=` instead of `==`

**Wrong:**
```caria
if (x = 5) {  // Assignment, not comparison!
    // ...
}
```

**Correct:**
```caria
if (x == 5) {  // Comparison
    // ...
}
```

#### 2. Missing Parentheses in Conditions

**Wrong:**
```caria
if x > 5 {  // Syntax error
    // ...
}
```

**Correct:**
```caria
if (x > 5) {
    // ...
}
```

#### 3. Forgetting `ret` Statement

**Wrong:**
```caria
rune main() -> int {
    say(42)
    // Missing return!
}
```

**Correct:**
```caria
rune main() -> int {
    say(42)
    ret 0
}
```

## Performance Tips

### 1. Minimize Loop Iterations

**Slower:**
```caria
// Checking every number from 2 to n
i := 2
while (i < n) {
    if (n % i == 0) {
        // ...
    }
    i := i + 1
}
```

**Faster:**
```caria
// Only check up to sqrt(n)
i := 2
while (i * i <= n) {
    if (n % i == 0) {
        // ...
    }
    i := i + 1
}
```

### 2. Use Early Exit

```caria
rune main() -> int {
    found := 0
    i := 0
    
    while (i < 1000) {
        if (i == 42) {
            found := 1
            i := 1000  // Exit loop early
        }
        i := i + 1
    }
    
    ret 0
}
```

### 3. Avoid Redundant Calculations

**Bad:**
```caria
while (i < limit) {
    result := (x * x) + (y * y)  // Recalculated every iteration
    i := i + 1
}
```

**Good:**
```caria
x_squared := x * x
y_squared := y * y

while (i < limit) {
    result := x_squared + y_squared
    i := i + 1
}
```

## Advanced Patterns

### State Machine

```caria
rune main() -> int {
    STATE_INIT := 0
    STATE_PROCESSING := 1
    STATE_DONE := 2
    
    state := STATE_INIT
    counter := 0
    
    while (state != STATE_DONE) {
        if (state == STATE_INIT) {
            say(1000)  // Init marker
            state := STATE_PROCESSING
        } else if (state == STATE_PROCESSING) {
            say(counter)
            counter := counter + 1
            
            if (counter >= 5) {
                state := STATE_DONE
            }
        }
    }
    
    ret 0
}
```

### Binary Search (on range)

```caria
rune main() -> int {
    target := 42
    left := 0
    right := 100
    
    while (left <= right) {
        mid := (left + right) / 2
        
        if (mid == target) {
            say(mid)  // Found
            ret 0
        } else if (mid < target) {
            left := mid + 1
        } else {
            right := mid - 1
        }
    }
    
    say(0)  // Not found
    ret 1
}
```

### GCD (Greatest Common Divisor)

```caria
rune main() -> int {
    a := 48
    b := 18
    
    while (b != 0) {
        temp := b
        b := a % b
        a := temp
    }
    
    say(a)  // 6
    ret 0
}
```

## Next Steps

- Read [API_REFERENCE.md](API_REFERENCE.md) for embedding API details
- Read [FFI_API.md](FFI_API.md) for standard library function reference
- Check [examples/agent_scripts/](examples/agent_scripts/) for more examples
- Join the community to share your Caria scripts

---

**Happy scripting with Caria!** 🚀
