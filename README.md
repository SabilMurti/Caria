# 🦀 Caria Language

**Fast & AI Token Friendly Programming Language**

[![Build Status](https://img.shields.io/github/actions/workflow/status/caristdlib/caria/build.yml?branch=main)](https://github.com/caristdlib/caria/actions)
[![Version](https://img.shields.io/github/v/release/caristdlib/caria)](https://github.com/caristdlib/caria/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Discord](https://img.shields.io/discord/1234567890?logo=discord&label=Discord)](https://discord.gg/caria)

## 🚀 About Caria

Caria is a modern programming language designed for speed and efficiency, with special optimization for AI token consumption. Built with Rust using Cranelift JIT compilation for zero-cost abstractions.

## ✨ Features

- ⚡ **Zero-Cost Abstractions**: Powered by Cranelift JIT compiler
- 🔥 **AI Token Optimized**: Minimize token usage without sacrificing code clarity
- 🎯 **Type Safe**: Compile-time safety with flexible type inference
- 📦 **Modular Architecture**: Plug-in support for FFI integrations
- 🔒 **Memory Safe**: No garbage collector, manual memory management optional
- 🌐 **Cross Platform**: Linux, macOS, Windows support

## 🛠️ Quick Start

### Installation

```bash
# Install from source
git clone https://github.com/caristdlib/caria.git
cd caria && cargo build --release

# Or download pre-built binary (coming soon)
curl -L https://github.com/caristdlib/caria/releases/latest/download/caria_cli \
  | tar -xzf - && sudo cp caria_cli /usr/local/bin/
```

### Your First Caria Program

Create `hello.cria`:
```caria
print("Hello, Caria!")

// Variables are typed automatically
var name := "World"
print("Hello, " + name + "!")
```

Run it:
```bash
caria run hello.cria
```

## 📖 Documentation

See [docs.carista.dev](https://docs.caristdlib.github.io) for complete documentation.

- [Language Reference](docs/language-reference.md)
- [Standard Library API](docs/stdlib/api.md)
- [Best Practices](docs/best-practices.md)
- [Contributing Guide](CONTRIBUTING.md)

## 🎯 Use Cases

### System Programming
```caria
import std::io

func main() {
    data := io.readAll("/proc/meminfo")
    print("Available RAM: " + data)
}
```

### AI Integration
```caria
import ai::neural

model NeuralNet {
    network: NeuralNetwork
    
    func predict(input: [float]) -> float {
        return self.network.forward(input)
    }
}
```

### Web Server
```caria
import net::http

func handler(req: HttpRequest) -> HttpResponse {
    ret HttpResponse{
        status: 200,
        body: "Welcome to Caria!"
    }
}

server := HttpServer.create(handler)
server.start(8080)
```

## 🏗️ Project Structure

```
Caria/
├── crates/
│   ├── caria_runtime   # Runtime with FFI bindings
│   ├── caria_codegen   # JIT code generation
│   └── caria_parser    # Lexer & parser
├── Caria-Stdlib        # Standard library
├── tests               # Test suite
└── docs                # Documentation
```

## 🤝 Contributing

We welcome contributions! Please read our [Contributing Guide](CONTRIBUTING.md) before submitting pull requests.

### Development Setup

```bash
# Clone repository
git clone https://github.com/caristdlib/caria.git
cd caria

# Build debug version
cargo build

# Run tests
cargo test

# Build release
cargo build --release
```

## 🔧 Configuration

### Compiler Flags

```bash
# Enable optimizations
caria build --optimize

# Enable debug symbols
caria build --debug

# Enable specific features
caria build --features jit,fast-io
```

## 📄 License

Caria is released under the MIT License. See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

Built with ❤️ for the Caria ecosystem. Thanks to all contributors and supporters!

---

**Ready to code faster and smarter with Caria!** 🚀

For questions or issues, please use [GitHub Issues](https://github.com/caristdlib/caria/issues).
