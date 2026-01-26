#!/bin/bash
cargo clean
cargo build --release --workspace
7z a ./build/fs-utils-windows-x64.zip ./target/release/*.exe ./bin