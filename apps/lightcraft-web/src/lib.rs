//! LightCraft in the browser: the same egui UI as the desktop app, compiled to WASM.
//!
//! Build with `cargo xtask web` (→ `target/web-site/`), then serve that folder over HTTP (see
//! `docs/web.md`). Differences from the desktop host:
//! - the library lives in browser storage (OPFS, or IndexedDB where OPFS can't be written from the
//!   main thread): catalog journal, presets, view state and prefs are mirrored in memory and
//!   flushed in the background ([`files`]); imported originals are stored by content hash
//!   ([`store`]); a new library starts with the procedural demo photos;
//! - renders run in Web Workers, each a second instance of this module ([`wire`], `workers`), and
//!   fall back to the main thread (one job per frame) if workers can't start;
//! - export downloads the file through the browser.
//!
//! `?bench` in the URL runs a scripted first-paint / slider-latency measurement and logs it to the
//! console (see [`bench`]) on the generated demo photos in an unsaved library (it implies `?store=memory`, so it never edits
//! the library in browser storage); `?store=idb` forces the IndexedDB backend, `?store=memory` disables
//! persistence and `?workers=0` renders on the main thread.
//! `?host=parent` speaks to a same-origin page framing LightCraft (see [`host`]): it hands
//! photos in, receives what would be downloaded and hears about unsaved changes. A page can also
//! host LightCraft in its own document, no iframe: `data-host="page"` on the canvas opts in, and
//! the same messages travel as DOM `CustomEvent`s on the canvas.
//!
//! Evora: `?host=evora` makes the page the engine of Evora's Revela (memory mode, the Evora host
//! protocol with the parent page `evora-host.json` admits, purges asked by the host only; see
//! `ehp`, `purge` and [`storage_names`]); the host's documents, saves, commands, exports and
//! picker are `host_docs`, its keys `host_keys` (EVORA-REV-29); its menus (what the host build masks)
//! and the catalog of the commands Evora may run are `host_menus` (EVORA-REV-33).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod automation;
pub mod backup;
pub mod bench;
// The web host's drop gate, also compiled natively so its queue handling has tests (the module is
// unused in a native, non-test library build and would otherwise be dead code there).
#[cfg(any(test, target_arch = "wasm32"))]
mod drop_gate;
pub mod files;
pub mod host;
pub mod storage_names;
pub mod store;
pub mod wire;

#[cfg(target_arch = "wasm32")]
mod backend;
#[cfg(target_arch = "wasm32")]
mod ehp;
#[cfg(target_arch = "wasm32")]
mod host_docs;
#[cfg(target_arch = "wasm32")]
mod host_keys;
#[cfg(target_arch = "wasm32")]
mod host_menus;
#[cfg(target_arch = "wasm32")]
mod purge;
#[cfg(target_arch = "wasm32")]
mod safety;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod workers;
