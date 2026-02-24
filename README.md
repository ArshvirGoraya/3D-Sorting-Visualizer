# 3D-Sorting-Visualizer

- Made in [Rust](https://rust-lang.org/) + [Bevy](https://bevy.org/)
- Made mostly for the web (wasm32-unknown-unknown), but can be compiled for x86 targets too (at least Windows).

## Building
- Use [Trunk](https://trunk-rs.github.io/trunk) to build for the web: 
    - `trunk build`: builds the wasm, html, css, js
        - Take a look at the "Trunk Build" task in tasks.json to see CLI command used for release builds
    - `trunk serve --open`: builds, starts up a server, and opens browser to local address
- The build is placed inside the "docs/" directory (used by github pages)
- If building for x86_64, just use `cargo build` instead of trunk (only windows has been tested).
