---
title: Installation
---

This guide is for reference only, please check the latest information on getting starting with
Substrate [here](https://docs.substrate.io/main-docs/install/).

This page will guide you through the **2 steps** needed to prepare a computer for **Substrate**
development. Since Substrate is built with
[the Rust programming language](https://www.rust-lang.org/), the first thing you will need to do is
prepare the computer for Rust development - these steps will vary based on the computer's operating
system. Once Rust is configured, you will use its toolchains to interact with Rust projects; the
commands for Rust's toolchains will be the same for all supported, Unix-based operating systems.

## Build dependencies

Substrate development is easiest on Unix-based operating systems like macOS or Linux. The examples
in the [Substrate Docs](https://docs.substrate.io) use Unix-style terminals to demonstrate how to
interact with Substrate from the command line.

### Ubuntu/Debian

Use a terminal shell to execute the following commands:

```bash
sudo apt update
# May prompt for location information
sudo apt install -y git clang curl libssl-dev llvm libudev-dev
```

### Arch Linux

Run these commands from a terminal:

```bash
pacman -Syu --needed --noconfirm curl git clang
```

### Fedora

Run these commands from a terminal:

```bash
sudo dnf update
sudo dnf install clang curl git openssl-devel
```

### OpenSUSE

Run these commands from a terminal:

```bash
sudo zypper install clang curl git openssl-devel llvm-devel libudev-devel
```

### macOS

> **Apple M1 ARM** If you have an Apple M1 ARM system on a chip, make sure that you have Apple
> Rosetta 2 installed through `softwareupdate --install-rosetta`. This is only needed to run the
> `protoc` tool during the build. The build itself and the target binaries would remain native.

Open the Terminal application and execute the following commands:

```bash
# Install Homebrew if necessary https://brew.sh/
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"

# Make sure Homebrew is up-to-date, install openssl
brew update
brew install openssl
```

### Windows

**_PLEASE NOTE:_** Native Windows development of Substrate is _not_ very well supported! It is
_highly_ recommend to use
[Windows Subsystem Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10) (WSL) and
follow the instructions for [Ubuntu/Debian](#ubuntudebian). Please refer to the separate
[guide for native Windows development](https://docs.substrate.io/main-docs/install/windows/).

## Rust developer environment

This guide uses <https://rustup.rs> installer and the `rustup` tool to manage the Rust toolchain.
First install and configure `rustup`:

```bash
# Install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Configure
source ~/.cargo/env
```

The Rust toolchain will use the rust-toolchain.toml file to determine the version of Rust to use and
targets to build.

```bash
rustup --version
```

## Test your set-up

Now the best way to ensure that you have successfully prepared a computer for Substrate development
is to follow the steps in
[our first Substrate tutorial](https://docs.substrate.io/tutorials/v3/create-your-first-substrate-chain/).

## Troubleshooting Substrate builds

Sometimes you can't get the Substrate node template to compile out of the box. Here are some tips to
help you work through that.

### WebAssembly compilation

Substrate uses [WebAssembly](https://webassembly.org) (Wasm) to produce portable blockchain
runtimes. You will need to configure your Rust compiler to use
[`nightly` builds](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html) to allow you to
compile Substrate runtime code to the Wasm target.

> There are upstream issues in Rust that need to be resolved before all of Substrate can use the
> stable Rust toolchain.
> [This is our tracking issue](https://github.com/paritytech/substrate/issues/1252) if you're
> curious as to why and how this will be resolved.
