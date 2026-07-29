# Cognitience PP — Native Windows host

Thin Win32 desktop shell that **does not use Electron**. It spawns the existing Rust backend (`cognition-pp.exe`), waits for `GET /api/health`, and loads the product UI in a **WebView2** window.

## Prerequisites

- Rust toolchain (MSVC)
- Microsoft Edge **WebView2** Runtime
- Backend: `cargo build --release` from `cognition-pp/`

## Build / Run

From `cognition-pp/`:

```bat
npm run native:build
npm run native
```

Binary: `native-host/target/release/cognition-pp-native.exe`  
Default port: **8789**

Headless:

```bat
set COGNITION_NATIVE_HEADLESS_SECS=20
native-host\target\release\cognition-pp-native.exe --headless
```

## Packaging

```bat
npm run dist
```

`dist/CognitiencePP_v*_win.zip`. macOS/Linux via CI (`.github/workflows/native.yml`).
