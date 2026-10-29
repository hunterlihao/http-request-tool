fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");

    // 图标资源：存在则嵌入，不存在则跳过（不阻塞编译）
    let ico = std::path::Path::new("assets/app.ico");
    if ico.exists() {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/app.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winres icon compile failed: {e}");
        }
    }
}
