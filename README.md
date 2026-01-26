A collection of command line tools used internally, cleaned up and refactored for public release. Use at own risk.

## fs-luajit-decompile
Decode and decompile LuaJIT bytecode files (FS19/FS22). Decompiler by marsinator358.
```sh
cargo run -p fs-luajit-decompile -- <input> [<output>] [-r] [-s]
```
```sh
cargo build --release -p fs-luajit-decompile
```

## fs-luau-decompile
Decode and decompile Luau bytecode files (FS25). Embeds the [medal](https://github.com/scfmod/medal) decompiler.

Supports reading directly from GAR/DLC archives:
```sh
# Single file from archive
fs-luau-decompile dataS.gar/scripts/main.l64

# Directory inside archive
fs-luau-decompile -r dataS.gar/scripts/vehicles/ -o ./output/

# Entire archive
fs-luau-decompile -r dataS.gar -o ./output/
```

```sh
cargo run -p fs-luau-decompile -- <input> [<output>] [-r] [-s] [-d] [--num-threads <n>]
```

```sh
cargo build --release -p fs-luau-decompile
```

## fs-shapes-unlock
Unlock .i3d.shapes files.
```sh
cargo run -p fs-shapes-unlock -- <input_file> [<output_path>] [-r] [-s]
```
```sh
cargo build --release -p fs-shapes-unlock
```

## fs-unpack
Extract GAR/DLC archives. Cross-platform, no external dependencies.
```sh
cargo run -p fs-unpack -- <archive> <output_path> [-s]
```
```sh
cargo build --release -p fs-unpack
```

## fs-xml-format
Parse XML files and output sane formatted XML.
```sh
cargo run -p fs-xml-format -- <input> [<output>] [-r] [-s] [-e] [-c <indent-char>] [-i <indent-size>]
```
```sh
cargo build --release -p fs-xml-format
```
