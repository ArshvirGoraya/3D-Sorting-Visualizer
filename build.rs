fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // https://docs.rs/winresource/latest/winresource/index.html#example
        let mut res = winresource::WindowsResource::new();
        res.set_icon("embedded_assets/favicon/favicon.ico");
        res.compile().unwrap();
    }
}
