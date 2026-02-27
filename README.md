# 3D-Sorting-Visualizer

## About
- Wanted to make something in [Rust](https://rust-lang.org/) and [Bevy](https://bevy.org/) and decided on this from watching [sorting algorithm videos](https://youtu.be/kPRA0W1kECg) which became viral for a time.
- Made mostly for the web (wasm32-unknown-unknown), but can be compiled for x86_64 targets too (at least Windows).

## Building
- Web Builds:
    - The "Github Release Build", "Local Release Build" and "Dev Build" tasks in tasks.json show all the required sequence of commands for building for the web.
        - Uses [Trunk](https://trunk-rs.github.io/trunk) and [esbuild](https://github.com/evanw/esbuild).
        - Some of the commands may only run on windows so if building on another OS, you will have to change them.
    - The build is placed inside the "docs/" directory because it's used by github pages.
- x86_64 builds:
    - Use `cargo build --release` (only windows has been tested as an x86 target).
