use notify::{Watcher, RecursiveMode, Result, Event};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

pub fn start_watcher(file_path: &str) -> Result<()> {
    println!("Watching '{}' for changes...", file_path);

    let path_to_watch = Path::new(file_path);
    let path_to_watch_cloned = path_to_watch.to_path_buf();
    
    // Store reference to running child process
    let current_child: Arc<parking_lot::Mutex<Option<std::process::Child>>> = Arc::new(parking_lot::Mutex::new(None));
    let current_child_clone = Arc::clone(&current_child);

    let compile_and_run = move || {
        println!("\n[Watcher] Change detected. Recompiling & restarting server...");
        
        // Terminate old process if active
        let mut child_guard = current_child_clone.lock();
        if let Some(mut child) = child_guard.take() {
            println!("[Watcher] Stopping previous server process...");
            let _ = child.kill();
            let _ = child.wait();
        }

        // Spawn new build/run process asynchronously
        let child_res = Command::new("cargo")
            .arg("run")
            .arg("-p")
            .arg("caria_cli")
            .arg("--")
            .arg("run")
            .arg(path_to_watch_cloned.to_str().unwrap())
            .spawn();

        match child_res {
            Ok(child) => {
                *child_guard = Some(child);
                println!("[Watcher] Server spawned successfully.");
            }
            Err(e) => {
                eprintln!("[Watcher] Failed to spawn compiler/server: {}", e);
            }
        }
    };

    // Run first execution
    compile_and_run();

    let compile_and_run_arc = Arc::new(tokio::sync::Mutex::new(compile_and_run));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event>| {
        match res {
            Ok(event) => {
                if event.kind.is_modify() {
                    let compile_and_run_clone = Arc::clone(&compile_and_run_arc);
                    rt.block_on(async {
                        let runner = compile_and_run_clone.lock().await;
                        (runner)();
                    });
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    })?;

    watcher.watch(path_to_watch, RecursiveMode::NonRecursive)?;

    // Keep watcher alive
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
