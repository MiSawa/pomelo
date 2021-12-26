# Pomelo

Pomelo is a WIP OS based on [mikanos](https://github.com/uchan-nos/mikanos).


## Build

```sh
cargo build
```

## Develop

You need `qemu` and `cargo-make` installed.

```sh
yay -Sy qemu
cargo install --force cargo-make
```

You can now run the boot loader with the task `qemu`

```sh
cargo make qemu
```

