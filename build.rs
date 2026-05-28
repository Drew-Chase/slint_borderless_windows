fn main() {
    let path = "titlebar.slint";
    println!("cargo:rerun-if-changed={}", path);

    slint_build::compile(path).unwrap();
}
