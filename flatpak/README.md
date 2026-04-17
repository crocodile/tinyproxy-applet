# Flatpak Notes

This repo includes a starter Flatpak manifest for `com.example.tinyproxy-applet`.

## Important limitation

COSMIC appears to launch this applet by executable name (`tinyproxy-applet`), not by Flatpak app ID.
That means a Flatpak package by itself may not be enough for the panel to discover and launch it.

If COSMIC cannot launch Flatpak-packaged applets directly, you will still need a small host-side wrapper:

```sh
#!/bin/sh
exec flatpak run com.example.tinyproxy-applet "$@"
```

installed somewhere on `PATH` as `tinyproxy-applet`.

## Service status from inside Flatpak

The applet detects Flatpak at runtime and uses:

```sh
flatpak-spawn --host systemctl is-active --quiet tinyproxy
```

This is why the manifest requests:

```text
--talk-name=org.freedesktop.Flatpak
```

## Cargo dependencies

The manifest is configured to build with:

```sh
cargo build --release --offline
```

To make that work, replace `flatpak/cargo-sources.json` with generated cargo sources, for example via `flatpak-cargo-generator`, or vendor dependencies another way.

## Typical build flow

```sh
flatpak-builder build-dir flatpak/com.example.tinyproxy-applet.yml
flatpak-builder --user --install --force-clean build-dir flatpak/com.example.tinyproxy-applet.yml
```
