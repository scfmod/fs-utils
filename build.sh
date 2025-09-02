#!/bin/bash
cargo build --release --workspace --exclude fs-unpack
cargo build --release -p fs-unpack --target i686-pc-windows-msvc

7z a ./build/fs-tools.zip ./target/release/*.exe ./target/i686-pc-windows-msvc/release/fs-unpack.exe ./bin