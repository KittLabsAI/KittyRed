# KittyAlpha

[中文版 README](./README_ZH.md)

KittyAlpha is a local-first desktop app for cross-exchange crypto market monitoring, AI recommendations, paper trading, and assistant-driven research. The frontend is built with React, Vite, and TypeScript. The backend is built with Rust and Tauri.

## Current surface

- Dashboard, Markets, Pair Detail, and Spread Monitor
- Recommendations, recommendation history, and audit views
- Portfolio, positions, and orders across `paper`, `real_read_only`, and `dual` modes
- Exchange, model, risk, prompt, account-mode, and notification settings
- Assistant Drawer with tool-backed market, portfolio, and recommendation context

## Key paths

- `src/`: frontend pages, components, stores, and bridge code
- `src-tauri/src/`: Tauri commands and backend domains
- `AGENTS.md`: repo-specific guidance for coding agents

## Development

```bash
npm install
npm test
npm run build
cd src-tauri && cargo test
npm run tauri -- dev
```

## Notes

- This branch focuses on the application source and runtime code.
