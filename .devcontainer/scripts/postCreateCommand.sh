#! /usr/bin/env bash

rustc --version
rustup update
rustup component add llvm-tools-preview
rustup target add thumbv6m-none-eabi
cargo install cargo-binutils