# Tinyproxy Applet

Small COSMIC panel applet that shows whether an existing `tinyproxy` service is running.
It does not install, create, or configure a proxy service; it only shows the current status if one is already present on the system.

## What It Does

- Shows a green icon when Tinyproxy is active
- Shows a red icon when Tinyproxy is inactive
- Shows a hover tooltip with the current status
- Polls `systemctl is-active tinyproxy` every 5 seconds

## Requirements

- Rust toolchain
- COSMIC desktop
- A `tinyproxy` systemd service on the host

## Build

```sh
cargo build --release
```

## Install

System-wide install:

```sh
sudo make install
```

If `tinyproxy-applet` is already installed in `/usr/bin` or `/usr/local/bin`, `make install`
updates that existing location so COSMIC keeps using the refreshed binary.

Manual uninstall from `/usr/local/bin`:

```sh
sudo make uninstall
```

## Make Targets

```sh
make build
make install
make uninstall
make clean
```

## Notes

- `make install` uses `/usr/local/bin`, which is the recommended place for locally built binaries.
- If an older system copy already exists in `/usr/bin`, `make install` will update that path so the panel does not keep launching the stale binary.
- COSMIC may launch one applet process per output or panel instance, so seeing multiple `tinyproxy-applet` processes can be normal.

## License

MIT. See [LICENSE](/home/z13bro/Code/tinyproxy-applet/LICENSE).
