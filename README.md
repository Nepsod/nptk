<div align="center">

# nptk (Nepsod Toolkit)

**Modern and Innovative UI Framework written in Rust**

*(Fork of the [Maycoon](https://github.com/maycoon-ui/maycoon) Toolkit)*
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

## License

This project uses a mixed licensing model:

### Permissive License (MIT/Apache-2.0)

The following crates are dual licensed under the [MIT license](LICENSE-MIT) and the [Apache License 2.0](LICENSE-APACHE):

- `nptk-core` - Core framework and utilities
- `nptk-theme` - Theme system
- `nptk-macros` - Procedural macros
- `nptk-widgets` - Core widget library (button, checkbox, container, icon, image, slider, text, etc.)

### Copyleft License (LGPL-3.0-only)

The following crates are licensed under the [GNU Lesser General Public License v3.0 only](LICENSE-LGPLv3):

- `nptk-widgets-extra` - Additional widgets (menu, progress, text input, sidebar, tabs, toolbar, etc.)
- `nptk-services` - Services layer (filesystem, bookmarks, thumbnails, etc.)

### Main Crate (`nptk`)

The main `nptk` crate facade is MIT/Apache-2.0, but by default includes optional LGPL-3.0 dependencies via the `lgpl-widgets` feature. To build a purely permissive version, disable this feature:

```toml
[dependencies]
nptk = { version = "0.5.0", default-features = false, features = ["macros"] }
```

Any contributions are, unless otherwise stated, licensed under the same terms as the crate they modify.
