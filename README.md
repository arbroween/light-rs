# Light
![screenshot](https://user-images.githubusercontent.com/3920290/81471642-6c165880-91ea-11ea-8cd1-fae7ae8f0bc4.png)

A lightweight text editor written in Rust and Lua, forked from [lite](https://github.com/rxi/lite).

## Overview
Light is a lightweight text editor aiming to be a fully open source and lighter alternative to VS Code. It is currently a personal project and far from being a serious alternative.

The project was forked from [lite](https://github.com/rxi/lite) and its original C code was converted into Rust using [C2Rust](https://github.com/immunant/c2rust). Refactoring the code into more safe and idiomatic Rust is WIP.

## Customization
Additional functionality can be added through plugins and colors themes.

Light should be compatible with the original lite themes and plugins so far:
* **[Get plugins](https://github.com/rxi/lite-plugins)** — Add additional functionality
* **[Get color themes](https://github.com/rxi/lite-colors)** — Add additional colors themes

The editor can also be customized by making changes to the [user module](data/user/init.lua).

## Building
Building this project requires [installing the Rust compiler and the Cargo package manager](https://www.rust-lang.org/tools/install).

Once the tools are installed, you can build and run the project yourself on Linux using `cargo run`.

Note that the project does not need to be rebuilt if you are only making changes to the Lua portion of the code.

## Contributing
Any additional functionality that can be added through a plugin should be done so as a plugin, after which a pull request to the [plugins repository](https://github.com/rxi/lite-plugins) can be made. Bug reports and bug fixes are welcome.

## License
This project is free software; you can redistribute it and/or modify it under the terms of the MIT license. See [LICENSE](LICENSE) for details.
