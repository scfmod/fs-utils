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
Decode and decompile Luau bytecode files (FS25). Using medal decompiler.
```sh
cargo run -p fs-luau-decompile -- <input> [<output>] [-r] [-s] [-d] [-l] [-t]
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
Unpack archive using ``defarm.dll``. Only compiling and running 32-bit version works.
```sh
cargo run -p fs-unpack --target i686-pc-windows-msvc -- <input_file> <output_path>
```
```sh
cargo build --release -p fs-unpack --target i686-pc-windows-msvc
```

## fs-unpack-dll
Extract ``defarm.dll`` from QuickBMS script file.
```sh
cargo run -p fs-unpack-dll -- <input_file> [<output_path>]
```
```sh
cargo build --release -p fs-unpack-dll
```

## fs-xml-format
Parse XML files and output sane formatted XML.
```sh
cargo run -p fs-xml-format -- <input> [<output>] [-r] [-s] [-c <indent-char>] [-i <indent-size>]
```
```sh
cargo build --release -p fs-xml-format
```