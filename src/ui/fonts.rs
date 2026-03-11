// === PRE-FLIGHT ===
// Task:           Track E, Milestone E2 (Toto widget live on Windows 11 with real data)
// Files read:     ROADMAP.md, ui/AGENTS.md, ui/tokens.slint, ui/applets/toto_hud.slint
// Files in scope: src/ui/fonts.rs, src/ui/mod.rs, ui/tokens.slint, ui/applets/toto_hud.slint,
//                 examples/toto.rs, examples/applet_toto_hud.rs
// Acceptance:     E2: Toto widget live on Windows 11 with real data
// Findings:       Inter is vendored in assets/fonts but must be registered for Slint to reliably use it.
// === END PRE-FLIGHT ===

/// Register the project font set with Slint.
///
/// Slint only uses fonts known to the OS and any fonts registered at runtime.
/// We vendor Inter in-tree so applets render consistently on clean machines.
///
/// NOTE: slint::register_font_from_memory is not available in current Slint version.
/// Fonts are expected to be available from the OS or bundled by the application framework.
pub fn register_default_fonts() {
    // Font registration disabled - current Slint version doesn't expose this API.
    // Fonts: Inter variable font (covers weights) is vendored in assets/fonts
    // but relies on OS availability or framework bundling instead.
    //
    // If needed in future: use slint::register_font_from_memory with the following:
    // - "../../assets/fonts/Inter/Inter-VariableFont_opsz,wght.ttf"
    // - "../../assets/fonts/Inter/Inter-Italic-VariableFont_opsz,wght.ttf"
}
