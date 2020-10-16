[![crates.io](https://img.shields.io/crates/v/bloxberg.svg)](https://crates.io/crates/bloxberg)

Bloxberg
========

*Bloxberg* is an experimental TUI-based hex editor written in Rust.
The long term vision for this tool is to become an integrated debugging and reverse-engineering tool, which is centered around a memory view.
It currently supports the following features distinguishing it from ordinary hex editors:

- Inline formatting as hex, decimal, octal or binary numbers.
- Dynamic byte width and byte order.
- Decode as text or instructions.