# CDKTR Python Bindings

This crate provides Python bindings for CDKTR using PyO3.

## Building

This crate is **excluded from the main workspace** to avoid bundling Python dependencies with the main CDKTR distribution.

To build the Python bindings:

```bash
cd ../../cdktr-py
./build.sh
```

Or manually:

```bash
cd ../../cdktr-py
uv run maturin develop --release
```

## Why Separate?

- Python bindings add PyO3 as a dependency
- Not needed for the core CDKTR CLI/TUI/server
- Keeps main builds fast and lean
- Only built when explicitly needed for Python integration

## Documentation

See the main Python package documentation in `cdktr-py/README.md`.
