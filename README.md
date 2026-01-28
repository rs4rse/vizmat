# vizmat

Molecule and crystal visualizer go brrr

`vizmat` is an experimental project to build a crystal structure visualizer in Rust using [Bevy](https://bevyengine.org/).

This project started as part of a rust learn and hack session at Paul Scherrer Institute — going line by line through code, learning Bevy and Rust fundamentals together, and gradually building up toward a minimal viable product.

This project is an educational experiment:

- We discuss Rust basics, Bevy concepts, and code structure as a group (mix of rust beginners and some are already fluent in rust but new to bevy/wasm).
- Each session recaps progress, explains new code, and sets up the next steps
- By the end, several contributors will have hands-on knowledge of Bevy and Rust from scratch, a good foundation for an open-source project

## Contributing & Development Setup

Contributions are welcome. This project uses Rust and Bevy. Familiarity with Rust tooling and basic CLI usage is assumed.

### Windows

* Install Rust via `rustup` (MSVC toolchain)
* Optional: Vulkan SDK

```powershell
cargo run
```

### Linux

Install Rust via `rustup`, then required system libraries.

Ubuntu:

```bash
sudo apt install build-essential pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-dev mesa-vulkan-drivers vulkan-utils
cargo run
```

### macOS

```bash
brew install cmake pkg-config
cargo run
```

(Bevy uses Metal; no Vulkan needed.)

## wasm

It require rustc target `wasm32-unknown-unknown` installed.
To run wasm version, install `trunk` and run it inside the binary folder `vizmat-app/`.

```console
cd vizmat-app
trunk serve
```

then open the link in your favorite browser. 

(Bevy on browser use wgpu)

## Roadmap

* [x] Initial Bevy setup
* [x] Basic crystal visualization (desktop)
* [x] Browser support via WASM
* [ ] Extend with interactivity and file parsing
* [x] Build community contributions

## Contributing

We are at an early stage and welcome contributions.
If you are new to Rust, Bevy, or visualization, this is the perfect playground.

## License

All contributions must retain this attribution.

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

