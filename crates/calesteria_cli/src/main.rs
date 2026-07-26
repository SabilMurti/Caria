use std::fs;
use std::path::Path;

fn cmd_new(name: &str) {
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("Error: Directory '{}' already exists.", name);
        std::process::exit(1);
    }

    // 1. Create Laravel-grade directory structure
    fs::create_dir_all(dir.join("app/controllers/Auth")).unwrap();
    fs::create_dir_all(dir.join("app/models")).unwrap();
    fs::create_dir_all(dir.join("app/middleware")).unwrap();
    fs::create_dir_all(dir.join("app/services")).unwrap();
    fs::create_dir_all(dir.join("app/jobs")).unwrap();
    fs::create_dir_all(dir.join("bootstrap")).unwrap();
    fs::create_dir_all(dir.join("config")).unwrap();
    fs::create_dir_all(dir.join("database/migrations")).unwrap();
    fs::create_dir_all(dir.join("routes")).unwrap();
    fs::create_dir_all(dir.join("views/layouts")).unwrap();
    fs::create_dir_all(dir.join("views/components")).unwrap();
    fs::create_dir_all(dir.join("public/css")).unwrap();
    fs::create_dir_all(dir.join("public/js")).unwrap();
    fs::create_dir_all(dir.join("storage/logs")).unwrap();
    fs::create_dir_all(dir.join("spells")).unwrap();
    fs::create_dir_all(dir.join("tests")).unwrap();

    // 2. Scaffold .env
    fs::write(dir.join(".env"), "APP_NAME=\"Calesteria Magic App\"\nAPP_ENV=\"development\"\nPORT=8080\nDB_URL=\"sqlite://app.db\"\n").unwrap();

    // 3. Scaffold caria.json (Composer/Package.json format)
    let caria_json_content = serde_json::json!({
        "$schema": "https://calesteria.dev/schema.json",
        "name": name,
        "type": "project",
        "description": "Full-Stack Application powered by Calesteria Framework.",
        "version": "1.0.0",
        "entry": "public/index.cria",
        "require": {
            "caria/framework": "^0.4.0"
        },
        "autoload": {
            "cria": {
                "App\\": "app/",
                "Database\\": "database/"
            }
        },
        "scripts": {
            "dev": "caria cast serve",
            "serve": "caria cast serve",
            "build": "caria build public/index.cria",
            "start": "./web"
        }
    });
    fs::write(dir.join("caria.json"), serde_json::to_string_pretty(&caria_json_content).unwrap()).unwrap();

    // 4. Scaffold bootstrap/app.cria
    fs::write(dir.join("bootstrap/app.cria"), r#"// Bootstrap Calesteria Framework Application
// Handles Application Configuration, Routing & Middlewares
rune bootstrap_app() -> int {
    ret 0
}
"#).unwrap();

    // 5. Scaffold Master Glyph Layout (views/layouts/app.glyph)
    fs::write(dir.join("views/layouts/app.glyph"), r#"<!DOCTYPE html>
<html lang='en'>
<head>
    <meta charset='UTF-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1.0'>
    <title>Calesteria Magic App</title>
    <script src='https://cdn.tailwindcss.com'></script>
    <script defer src='https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js'></script>
    <script src='https://unpkg.com/htmx.org@1.9.10'></script>
</head>
<body class='bg-gray-950 text-gray-100 min-h-screen flex flex-col items-center justify-center'>
    <main class='w-full max-w-xl p-6'>
        {{SLOT}}
    </main>
</body>
</html>"#).unwrap();

    // 6. Scaffold Home View (views/home.glyph)
    fs::write(dir.join("views/home.glyph"), r#"<div class='bg-gray-900 border border-gray-800 rounded-3xl p-8 shadow-2xl text-center' x-data='{ count: 0 }'>
    <h1 class='text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 to-purple-400 mb-2'>
        ✨ Welcome to Calesteria Magic App
    </h1>
    <p class='text-sm text-gray-400 mb-6'>
        Powered by Caria AOT Compiler Engine & Glyph Template System
    </p>

    <div class='flex items-center justify-center space-x-4 mb-6'>
        <button @click='count--' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>-</button>
        <span class='text-3xl font-mono text-indigo-400' x-text='count'>0</span>
        <button @click='count++' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>+</button>
    </div>

    <div class='text-xs text-gray-500 font-mono border-t border-gray-800 pt-4'>
        Alpine.js Micro-State + HTMX SSR Ready
    </div>
</div>"#).unwrap();

    // 7. Scaffold Welcome View (views/welcome.glyph)
    fs::write(dir.join("views/welcome.glyph"), r#"<!DOCTYPE html>
<html lang='en'>
<head>
    <meta charset='UTF-8'>
    <meta name='viewport' content='width=device-width, initial-scale=1.0'>
    <title>Calesteria Magic Framework</title>
    <script src='https://cdn.tailwindcss.com'></script>
    <script defer src='https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js'></script>
    <script src='https://unpkg.com/htmx.org@1.9.10'></script>
</head>
<body class='bg-gray-950 text-gray-100 min-h-screen flex flex-col items-center justify-center p-4'>
    <div class='bg-gray-900 border border-gray-800 rounded-3xl p-8 text-center max-w-md w-full shadow-2xl' x-data='{ count: 0 }'>
        <h1 class='text-3xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 to-purple-400 mb-3'>
            ✨ Welcome to Calesteria
        </h1>
        <p class='text-xs text-gray-400 mb-6'>
            Laravel-Grade Architecture — 100% Native AOT Caria Server
        </p>
        <div class='flex items-center justify-center space-x-4 mb-6'>
            <button @click='count--' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>-</button>
            <span class='text-3xl font-mono text-indigo-400' x-text='count'>0</span>
            <button @click='count++' class='px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded-xl text-white font-bold transition'>+</button>
        </div>
        <div class='text-xs text-gray-500 font-mono border-t border-gray-800 pt-4'>
            Rendered via view("welcome") • HTMX + Alpine.js Loaded
        </div>
    </div>
</body>
</html>"#).unwrap();

    // 8. Scaffold Web Routes (routes/web.cria)
    fs::write(dir.join("routes/web.cria"), r#"get "/" {
    ret view("welcome")
}
"#).unwrap();

    // 9. Scaffold API Routes (routes/api.cria)
    fs::write(dir.join("routes/api.cria"), r#"get "/api/health" {
    ret "{\"status\":\"ok\",\"engine\":\"Caria Native AOT\"}"
}
"#).unwrap();

    // 10. Scaffold Public Entrypoint (public/index.cria)
    fs::write(dir.join("public/index.cria"), r#"// Public Entrypoint for Calesteria Framework Application
// Routes are defined in routes/web.cria & routes/api.cria

rune main() -> int {
    ret 0
}
"#).unwrap();

    // 11. Scaffold Project Root `cast` Script File (Laravel `artisan` equivalent)
    let cast_script = r#"// Calesteria Framework Console Script (Laravel Artisan Equivalent)
// Handles `caria cast serve`, `caria cast make:controller`, `caria cast make:model`

rune main() {
    say("✨ Calesteria Framework Console")
}
"#;
    let cast_path = dir.join("cast");
    fs::write(&cast_path, cast_script).unwrap();

    // 12. Scaffold package.json (HTMX + Alpine.js + TailwindCSS + Vite)
    let package_json_content = serde_json::json!({
        "private": true,
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "vite build"
        },
        "devDependencies": {
            "alpinejs": "^3.14.0",
            "htmx.org": "^1.9.12",
            "tailwindcss": "^3.4.0",
            "vite": "^5.0.0"
        }
    });
    fs::write(dir.join("package.json"), serde_json::to_string_pretty(&package_json_content).unwrap()).unwrap();

    println!("✨ Created Calesteria Full-Stack Magic App '{name}'!");
    println!("  cd {name}");
    println!("  caria cast serve    (or ./cast serve)");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("✨ Calesteria Framework Installer CLI 0.4.0");
        println!("Usage:");
        println!("  calesteria new <app-name>   Scaffold a new Calesteria magic MVC project");
        println!("  calesteria --version        Print version info");
        return;
    }

    match args[1].as_str() {
        "new" => {
            if args.len() < 3 {
                eprintln!("Usage: calesteria new <project-name>");
                std::process::exit(1);
            }
            cmd_new(&args[2]);
        }
        "--version" | "-v" | "version" => {
            println!("Calesteria Framework Installer v0.4.0");
        }
        other => {
            eprintln!("Unknown calesteria command: '{}'", other);
            std::process::exit(1);
        }
    }
}
