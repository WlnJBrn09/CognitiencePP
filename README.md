# Cognition PP

Local-first **presentation** app with an Apple-inspired liquid-glass UI and a **Rust** backend.

Open and display **pdf**, **png**, and **pptx** files from your Documents folder. Insert images, PDF pages, video, and audio onto slides. Nothing is uploaded to the cloud.

## Requirements

- Rust 1.75+ (`cargo`)
- Node.js 18+ (for Electron packaging)
- A modern browser (Chrome, Edge, Firefox, Safari)

## Desktop app (Electron)

```bash
cargo build --release
npm install
npm run dist
```

Output: `dist/CognitiencePP_v1.0.0.exe` (portable)

## Run (dev)

```bash
cargo run
```

Then open **http://127.0.0.1:8789**

```bash
npm run electron:dev   # Electron shell + release backend
```

| Variable | Default | Meaning |
| --- | --- | --- |
| `PORT` | `8789` | HTTP port (localhost only) |
| `COGNITION_DATA_DIR` | `./documents` | Saved presentation JSON store |
| `COGNITION_STATIC_DIR` | `./static` | Frontend assets |
| `COGNITION_DOCS_DIR` | user Documents | Folder scanned for pdf / png / pptx |

## Features

- Clean presentation canvas (no Google-style Share / Upgrade / menu chrome)
- Left sidebar: **New Presentation**, **Open Presentation**, and live list of data files
- Correct open + display for **pdf**, **png**, and **pptx**
- Insert menu (PlusSquare): images (svg/png/jpeg/pdf pages), video, audio
- Fonts, page color, text color, highlight, underline, strikethrough, super/subscript
- Title / subtitle slides, text boxes, monochrome liquid-glass chrome
- Dark / light theme

## Project layout

```
build/          App icons
documents/      Local presentation JSON
electron/       Electron shell
scripts/        Icon builder
src/            Rust backend (Axum)
static/         Frontend (HTML/CSS/JS + logo)
```
