#!/bin/bash
cargo build --release --workspace
7z a ./build/fs-tools.zip ./target/release/*.exe ./bin