//! Example demonstrating CariaEngine embedding API usage

use caria_runtime::CariaEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Caria Embedding API Demo ===\n");
    
    // Create engine
    let mut engine = CariaEngine::new()?;
    println!("✓ CariaEngine created\n");
    
    // Test 1: Hello World
    println!("Test 1: Hello World");
    let script1 = r#"
        rune main() -> int {
            say("Hello from embedded Caria!")
            ret 42
        }
    "#;
    let result1 = engine.eval(script1)?;
    println!("  Exit code: {}\n", result1);
    
    // Test 2: Arithmetic
    println!("Test 2: Basic Arithmetic");
    let script2 = r#"
        rune main() -> int {
            x := 5
            y := 10
            sum := x + y
            diff := x - y
            say(sum)
            say(diff)
            ret 0
        }
    "#;
    let result2 = engine.eval(script2)?;
    println!("  Exit code: {}\n", result2);
    
    // Test 3: Control Flow
    println!("Test 3: While Loop");
    let script3 = r#"
        rune main() -> int {
            sum := 0
            i := 0
            while (i < 5) {
                sum := sum + i
                i := i + 1
            }
            say(sum)  // Should print 10
            ret 0
        }
    "#;
    let result3 = engine.eval(script3)?;
    println!("  Exit code: {}\n", result3);
    
    // Test 4: If Statement
    println!("Test 4: If/Else");
    let script4 = r#"
        rune main() -> int {
            score := 85
            if (score >= 60) {
                say("Passed")
            } else {
                say("Failed")
            }
            ret 0
        }
    "#;
    let result4 = engine.eval(script4)?;
    println!("  Exit code: {}\n", result4);
    
    println!("=== All Tests Completed ===");
    Ok(())
}
