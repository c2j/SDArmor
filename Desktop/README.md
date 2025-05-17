# SDChat Security Scanner Desktop Client

## Overview

The SDChat Security Scanner is a high-performance security vulnerability scanning tool designed to detect potential security issues in source code. This desktop client provides a graphical user interface for running scans, viewing results, and managing security rules.

## Features

- **Interactive 3D visualization** of vulnerability hotspots
- **Real-time scanning** with progress monitoring
- **Rule-based detection** of security vulnerabilities
- **Multi-threaded scanning engine** for high performance
- **Detailed reports** with code snippets and severity information
- **Cross-platform support** for Windows, macOS, and Linux

## Requirements

- Rust 1.70.0 or later
- Platform-specific dependencies for GUI rendering (Metal on macOS, Vulkan on Linux/Windows)
- Optional: Hyperscan library for regex acceleration

## Installation

### Install Rust

If you don't have Rust installed, you can install it using rustup:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Install Dependencies

#### Hyperscan (Optional but Recommended)

Hyperscan provides significant performance improvements for pattern matching:

**macOS:**
```sh
brew install hyperscan
```

**Ubuntu/Debian:**
```sh
sudo apt-get install libhyperscan-dev
```

**Fedora/RHEL:**
```sh
sudo dnf install hyperscan-devel
```

### Build and Run

Clone the repository and build the application:

```sh
git clone https://github.com/your-organization/sdchat-sc.git
cd SDChat-SC/Desktop
```

#### Standard Build (without Hyperscan):

```sh
cargo build --release
```

#### Build with Hyperscan acceleration:

```sh
cargo build --release --features hyperscan_engine
```

#### Run the application:

```sh
cargo run --release
```

## Usage

1. **Select a directory or project** to scan using the "Target Path" section
2. **Choose rule sets** to apply during scanning
3. **Click "Start Scan"** to begin the security analysis
4. View results in the **Dashboard** (real-time) and **Vulnerability Details** tabs
5. Export reports in various formats (JSON, HTML, Markdown, PDF)

## Configuration

The application stores configuration in the following locations:

- **Windows**: `%APPDATA%\SDChat-Scanner\config.json`
- **macOS**: `~/Library/Application Support/SDChat-Scanner/config.json`
- **Linux**: `~/.config/sdchat-scanner/config.json`

Key configuration options:

- **Server URL**: API endpoint for rule updates and report submission
- **Thread Count**: Number of parallel scanning threads (default: number of CPU cores)
- **Ignore Patterns**: Regular expressions for files/directories to ignore
- **API Key**: Authentication key for server communication
- **Proxy URL**: Optional HTTP proxy for server communication
- **Network Timeout**: Connection timeout in seconds (default: 30)
- **Certificate Validation**: Enable/disable TLS certificate validation

## Troubleshooting

### Common Issues

#### Missing Hyperscan Library

If you see an error about missing `libhs`:

1. Install Hyperscan using the instructions above
2. Set the PKG_CONFIG_PATH environment variable if needed:
   ```sh
   export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
   ```
3. Alternatively, build without Hyperscan: `cargo build --release`

#### GPU Rendering Issues

If you encounter graphics rendering problems:

1. Try setting the environment variable: `WGPU_BACKEND=vulkan` (or `metal` on macOS)
2. Update your graphics drivers
3. Run with software rendering: `WGPU_POWER_PREF=low cargo run`

## License

Copyright © 2023 SDChat Security Scanner Team. All rights reserved.