fn main() {
    slint_build();
    icon();
}

fn slint_build() {
    let path = "ui/main_window.slint";
    println!("cargo:rerun-if-changed={}", path);
    unsafe {
        std::env::set_var("SLINT_BACKEND", "winit-skia");
    }
    slint_build::compile(path).unwrap();
}

fn icon() {
    #[cfg(windows)]
    {
        let path = "../../res/icons/icon.ico";
        println!("cargo:rerun-if-changed={}", path);
        let mut res = winresource::WindowsResource::new();
        res.set_icon(path);

        res.compile().unwrap();
    }
}
