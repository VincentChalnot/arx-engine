// Embeds assets/generated/keres.ico as a Win32 PE resource on the `gui`
// binary, so the .exe file itself carries the icon (Explorer, the taskbar,
// Alt-Tab) with no runtime API call — see the winres build-dependency note
// in Cargo.toml and the removed Windows branch in src/gui/platform_icon.rs.
// A no-op everywhere else: non-Windows targets, and the `keres`/`server`
// binaries, which build without the `gui` feature.
fn main() {
    #[cfg(all(windows, feature = "gui"))]
    {
        winres::WindowsResource::new()
            .set_icon("assets/generated/keres.ico")
            .compile()
            .expect("failed to embed the Windows icon resource");
    }
}
