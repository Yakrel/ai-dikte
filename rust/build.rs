fn main() {
    println!("cargo:rerun-if-changed=../ai-dikte.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("../ai-dikte.ico")
        .set("FileDescription", "AI Dikte")
        .set("ProductName", "AI Dikte")
        .compile()
        .expect("Cannot compile Windows application icon");
}
