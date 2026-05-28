fn main() {
    let path = "ui/main_window.slint";
    println!("cargo:rerun-if-changed={}", path);

    slint_build::compile(path).unwrap();
}
