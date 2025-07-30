# development instructions

## requirements

- Rust: https://rustup.rs/
- Dependencies:

```sh
# build tools apart from Rust
build-essential pkg-config

# egui (https://github.com/emilk/egui)
libclang-dev libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

# other dependencies
librust-alsa-sys-dev libssl-dev
```

## prerequisites

create a dummy `glass` crate:

```sh
./crates/fake-glass.sh
```
