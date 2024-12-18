# Celeris Browser Engine

Celeris is a high-performance, ground-up browser engine written in Rust, designed to redefine speed, reliability, and efficiency in web browsing.

## Features

- **Pure Rust Implementation**: Built from the ground up in Rust for maximum safety and performance
- **Modern Architecture**: Clean separation of concerns with modular design
- **HTML5 Parser**: Custom HTML parser with robust error handling
- **CSS3 Support**: Advanced CSS parsing and styling engine
- **Network Stack**: Custom networking layer with HTTP/HTTPS support
- **Rendering Engine**: Flexible rendering pipeline with both headless and windowed modes
- **JavaScript Engine**: (In development) Custom JS engine for web interactivity
- **Developer Tools**: Built-in debugging and logging capabilities

## Project Structure

```
src/
├── css/           # CSS parsing and styling engine
├── dom/           # DOM tree implementation
├── html/          # HTML parsing and tokenization
├── javascript/    # JavaScript engine (WIP)
├── networking/    # HTTP/HTTPS client implementation
├── rendering/     # Layout and painting engine
└── logger.rs      # Logging infrastructure
```

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Cargo package manager

### Installation

1. Clone the repository:
```bash
git clone https://github.com/SkuldNorniern/celeris.git
cd celeris
```

2. Build the project:
```bash
cargo build --release
```

### Usage

Run Celeris with a URL:
```bash
cargo run --release
```

Then enter a URL when prompted, or press Enter to load the default page (example.com).

## Configuration

Celeris can be configured through the `BrowserConfig` struct:

```rust
let config = BrowserConfig {
    headless: true,  // Run in headless mode
    debug: true,     // Enable debug output
};
```

## Features in Detail

### Networking

- Support for HTTP/1.1 and HTTPS
- Custom TLS implementation using rustls
- Efficient async I/O with tokio

### DOM Implementation

- Complete DOM tree representation
- Efficient node manipulation
- Event handling system

### CSS Engine

- Selector parsing and matching
- Style computation and inheritance
- Specificity calculation

### Rendering Pipeline

- Layout engine with block and inline formatting
- Painting system with both buffer and window targets
- Support for headless rendering

## Development Status

Celeris is currently in active development. The following features are planned or in progress:

- [ ] Complete JavaScript engine integration
- [ ] Full CSS3 support
- [ ] Web Workers support
- [ ] WebAssembly support
- [ ] GPU acceleration
- [ ] Extended developer tools
