# Installation

cdktr is distributed as a single, self-contained binary with no external dependencies required. This makes installation straightforward on any platform.

## Prerequisites

None! cdktr is completely self-contained and doesn't require Rust or any other runtime to be installed.

## Installation Methods

### From Pre-built Binaries

Pre-built binaries for Linux, macOS, and Windows are available from the [GitHub Releases](https://github.com/nicelgueta/cdktr/releases) page.

Download the appropriate binary for your platform and add it to your PATH:

```bash
# Example for Linux/macOS
chmod +x cdktr
sudo mv cdktr /usr/local/bin/
```

### From Source (Recommended for Development)

If you have Rust installed, you can build cdktr from source:

```bash
git clone https://github.com/nicelgueta/cdktr.git
cd cdktr
cargo build --release
```

The compiled binary will be available at `target/release/cdktr`.


### Verify Installation

Verify that cdktr is installed correctly:

```bash
cdktr --version
```

You should see the version number displayed.

## Next Steps

Now that you have cdktr installed, let's explore the [Quick Start Guide](./quickstart.md) to get your first workflow running!
