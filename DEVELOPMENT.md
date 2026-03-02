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

# puffin-profiler
libglib2.0-dev libatk1.0-dev libgtk-3-dev
```

## prerequisites

create a dummy `glass` crate:

```sh
./crates/fake-glass.sh
```

## profiling

to enable performance profiling with [puffin](https://github.com/EmbarkStudios/puffin), build with the `puffin` feature:

```sh
# client
cargo run --release --features puffin

# visual tests
cargo run -p visual_tests --features puffin --release -- \
    --count 100000 --mode target --ch 1 --dm 0
```

to collect the profile and exit automatically, run:

```
cargo run -p visual_tests --features puffin --release -- \
    --count 150000 --mode target --ch 1 --dm 0 \
    --bench-duration 10 --output before.puffin
```

to view the collected profile, run:

```
cargo install puffin_viewer
puffin_viewer before.puffin
```
