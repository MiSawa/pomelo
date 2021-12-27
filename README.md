# Pomelo

Pomelo is a WIP OS based on [mikanos](https://github.com/uchan-nos/mikanos).


## Build

You need `cargo-make` to build this OS.

```sh
cargo install --force cargo-make
cagro make build
```

Instead, you can run `cargo build` on `crates/{bootloader,kernel}`.

## Develop

You need `qemu` and `cargo-make` installed.

```sh
yay -Sy qemu
cargo install --force cargo-make
```

You can now boot the OS with the task `qemu`

```sh
cargo make qemu
```

