//! `mpe_plugin_main!` entry-point macro.
//!
//! Wires a plugin's node implementations into the runtime event loop with a
//! single `main`. Declarative macro, ~15-30 lines. Implementation lands in
//! M5 (frontend_port field DROPPED per decision A4 inline-only).
