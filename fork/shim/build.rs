fn main() {
    #[cfg(windows)]
    {
        // Give the shim (installed as "Modrinth App.exe") the ByteLauncher icon
        // so existing shortcuts show the right artwork. The icon is cosmetic, so
        // this is best-effort: never fail the build if the resource compiler is
        // unavailable.
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../apps/app/icons/icon.ico");
        let _ = res.compile();
    }
}
