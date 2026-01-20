<div align="center">

# nptk (Nepsod Toolkit)
</div>

Private Cursor AI testing field, as I'd call it for now.
Don't use it in any serious project (until you know what you're doing?).
It's going to be changing a lot.
And to be focused around my personal projects (maybe even a whole Desktop Environment some day?).

## Features

- ⚙️ **Made in [Rust](https://www.rust-lang.org)**
    - We believe Rust is perfect for performance critical applications and efficient memory management.
- 🛠️ **Cross-platform**
    - As a desktop-focused UI framework, nptk is compatible with Windows, Linux, macOS and other Unix-like operating systems.
- 🎨 **Customizable**
    - nptk provides a variety of widgets and themes to customize the look and feel of your application with ease.
- 🚀 **Lightning Fast**
    - Your application will start up fast and run even faster.
    - The UI uses [vello](https://github.com/linebender/vello) for GPU-accelerated rendering and top performance.
- 📦 **Modular**
    - Every crate and feature is optional, so you can only enable what you really need and avoid unnecessary
      dependencies or features.
    - Build widgets using components or using raw vector graphics.

## Getting Started

If you are new to [Rust](https://www.rust-lang.org), we recommend learning the [basics](https://www.rust-lang.org/learn)
first.

### Controlling Log Verbosity

NPTK uses the `log` crate for logging. By default, the framework logs at `info` level and above. To control log verbosity, use the `RUST_LOG` environment variable:

```bash
# Show only errors and warnings
RUST_LOG=warn cargo run

# Show info, warnings, and errors (default)
RUST_LOG=info cargo run

# Show debug messages (verbose)
RUST_LOG=debug cargo run

# Show trace messages (very verbose, includes per-frame logs)
RUST_LOG=trace cargo run

# Filter by crate
RUST_LOG=nptk_core=debug,my_app=info cargo run

# Filter by module
RUST_LOG=nptk_core::app::handler=debug cargo run
```

Most verbose per-frame logging is at the `trace` level, so it won't appear unless explicitly enabled.

## License

This project uses a mixed licensing model:

### Permissive License (MIT/Apache-2.0)

The following crates are dual licensed under the [MIT license](LICENSE-MIT) and the [Apache License 2.0](LICENSE-APACHE):

- `nptk-widgets` - Core widget library (button, checkbox, container, icon, image, slider, text, etc.)

### Copyleft License (LGPL-3.0-only)

The following crates are licensed under the [GNU Lesser General Public License v3.0 only](LICENSE-LGPLv3):

- `nptk-widgets-extra` - Additional widgets (menu, progress, text input, sidebar, tabs, toolbar, etc.)
- `nptk-services` - Services layer (filesystem, bookmarks, thumbnails, etc.)

### Mixed License (per-file)

The following crates are SPDX-licensed on the per-file basis. Either under the [MIT license](LICENSE-MIT) and [Apache License 2.0](LICENSE-APACHE), or under the [GNU Lesser General Public License v3.0](LICENSE-LGPLv3).

- `nptk-core` - Core framework and utilities
- `nptk-macros` - Procedural macros

Any contributions are, unless otherwise stated, licensed under the same terms as the crate they modify.
