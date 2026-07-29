import re

with open("crates/caria_cli/src/main.rs", "r") as f:
    content = f.read()

# We want to replace everything from `let runtime_c = r#"` down to `let _ = fs::remove_file(&temp_runtime_c_path);`
# with the new linker code.

start_str = '    // Write runtime helper C source on the fly'
end_str = '    let _ = fs::remove_file(&temp_runtime_c_path);'

start_idx = content.find(start_str)
end_idx = content.find(end_str) + len(end_str)

if start_idx != -1 and end_idx != -1:
    new_code = """    // Find the directory where this caria_cli executable lives
    let current_exe = std::env::current_exe().expect("Failed to get current executable path");
    let target_dir = current_exe.parent().expect("Failed to get parent directory").to_str().unwrap();

    // Invoke CC (system compiler / linker) linking against the Rust Native Runtime
    let status = std::process::Command::new("cc")
        .arg("-no-pie")
        .arg("-o")
        .arg(&output_bin_path)
        .arg(&temp_obj_path)
        .arg(format!("-L{}", target_dir))
        .arg("-lcaria_runtime")
        .arg("-lpthread")
        .arg("-ldl")
        .arg("-lm")
        .status();

    // Clean up temporary files
    let _ = std::fs::remove_file(&temp_obj_path);"""
    
    modified = content[:start_idx] + new_code + content[end_idx:]
    with open("crates/caria_cli/src/main.rs", "w") as f:
        f.write(modified)
    print("Linker updated successfully!")
else:
    print("Failed to find start or end strings.")
