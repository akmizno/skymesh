# skymesh

A simple, cross-platform mesh viewer built with Rust and WebGPU.

## Usage

### Native

Run the native desktop application:

```bash
cargo run --release
```

### Web assembly (wasm).

To run the browser version locally, you will need [Trunk](https://trunkrs.dev/):

```bash
# Install Trunk if you haven't already.
cargo install trunk

# Serve the application.
trunk serve --release
```

Once started, open http://localhost:8080 in your browser.
