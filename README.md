# AriaDesk

A desktop frontend for [aria2](https://aria2.github.io/) for **Windows, macOS and Linux**, built with [Tauri 2](https://tauri.app/), plus a static **web** build.

AriaDesk pairs an AngularJS frontend with a Rust backend built on Tauri. Torrent parsing lives in a shared Rust crate that runs natively on the desktop and as WebAssembly in the browser.

## Features

- Desktop features:
  - aria2 RPC over HTTP and WebSocket made from the backend, so CORS does not apply
  - Tray icon (Windows/Linux), close to tray, native notifications
  - Default magnet link handler, `.torrent` / `.metalink` file associations
  - Single instance: files and links opened later go to the running window
  - Run a command at startup (for example `aria2c`), drag and drop of files and links
  - Window size and position are remembered
- ~6 MB packages (deb/rpm/AppImage, msi/nsis, dmg)
- Web build: static files you can serve from any HTTP server

## Layout

| Path | Contents |
|---|---|
| `app/` | Frontend (AngularJS) |
| `app/scripts/native/bridge.js` | Emulates Electron's `ipcRenderer` for the Tauri and web backends |
| `app/wasm/` | Generated wasm bindings of `ariadesk-core` |
| `crates/ariadesk-core/` | Shared Rust logic (bencode, torrent info) |
| `src-tauri/` | Tauri backend (IPC channels, tray, menus, WebSocket, HTTP, startup command) |
| `scripts/` | Frontend and wasm build scripts |

## Building

Requirements: Node.js 18+, Rust (stable) with the `wasm32-unknown-unknown` target, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS (on Linux: `webkit2gtk-4.1`, `libayatana-appindicator3`).

```sh
npm install

# optional: regenerate app/wasm after changing crates/ariadesk-core
cargo install wasm-bindgen-cli --version <version in Cargo.lock>
npm run build:wasm

npm run dev          # run the desktop app in development mode
npm run build        # build installers into target/release/bundle
npm run build:web    # build the web version into dist-web/
```

## Command line

```
ariadesk [file/url] [-d|--development] [-m|--minimal]
```

- `file/url`: a `.torrent` / `.metalink` file or a URL (including `magnet:`) to open in the new task page
- `-d`: enable debug mode (F12 opens the developer tools)
- `-m`: start hidden in the tray

## Web version

Serve `dist-web/` over HTTP and open `app/index.html`. The browser talks to aria2 directly, so start aria2 with `--enable-rpc --rpc-allow-origin-all`. Settings that need the desktop (tray, startup command, magnet handler) are hidden.

## License

GNU Affero General Public License v3.0 only (AGPL-3.0-only). See [LICENSE](LICENSE).
