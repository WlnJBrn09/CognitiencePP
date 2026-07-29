# Cognition PP

Local-first presentation app with liquid-glass UI and a **Rust** backend.

## Desktop app (native)

No Electron. WebView2 / WebKit host + local Rust backend.

```bash
npm run native:build
npm run native
npm run dist
```

Port **8789**. Package: `dist/CognitiencePP_v*_win.zip`

## Dev server

```bash
cargo run
```

http://127.0.0.1:8789

## License

MIT