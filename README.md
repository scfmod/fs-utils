A collection of command line tools used internally, cleaned up and refactored for public release. Use at own risk.



## fs-luau-decompile

```
Usage: fs-luau-decompile <input> [<output>] [-r] [-s] [-d] [--num-threads <num-threads>]

Decode and decompile Luau .l64 bytecode files

Positional Arguments:
  input             path to input file/folder
  output            path to output file/folder (optional)

Options:
  -r, --recursive   recursive mode if folder input
  -s, --silent      suppress output
  -d, --decode-only only decode files
  --num-threads     set thread pool size when processing folders (0 = auto)
```

Decode and decompile Luau bytecode files (FS25). Embeds the [medal](https://github.com/scfmod/medal) decompiler.

Supports reading directly from GAR/DLC archives:
```sh
# Single file from archive
fs-luau-decompile dataS.gar/scripts/main.l64

# Directory inside archive
fs-luau-decompile -r dataS.gar/scripts/vehicles/ ./output/

# Entire archive
fs-luau-decompile -r dataS.gar ./output/
```



```sh
cargo run -p fs-luau-decompile -- <input> [<output>] [-r] [-s] [-d] [--num-threads <n>]
```

```sh
cargo build --release -p fs-luau-decompile
```

## fs-luajit-decompile

```
Usage: fs-luajit-decompile <input> [<output>] [-r] [-s] [--num-threads <num-threads>]

Decode and decompile LuaJIT .l64 bytecode files

Positional Arguments:
  input             path to input file/folder
  output            path to output file/folder (optional)

Options:
  -r, --recursive   recursive mode if folder input
  -s, --silent      suppress output
  --num-threads     set thread pool size when processing folders (0 = auto)
```

Decode and decompile LuaJIT bytecode files (FS19/FS22). Decompiler by marsinator358.

```sh
# Single file
fs-luajit-decompile scripts/main.l64
fs-luajit-decompile scripts/main.l64 ./decompiled/main.lua

# Directory
fs-luajit-decompile -r scripts/ ./decompiled/
```



```sh
cargo run -p fs-luajit-decompile -- <input> [<output>] [-r] [-s]
```
```sh
cargo build --release -p fs-luajit-decompile
```

## fs-shapes-unlock

```
Usage: fs-shapes-unlock <input> [<output>] [-r] [-s]

Unlock .i3d.shapes files

Positional Arguments:
  input             path to input file/folder
  output            path to output file/folder (optional)

Options:
  -r, --recursive   recursive mode if folder input
  -s, --silent      suppress output
```

```sh
cargo run -p fs-shapes-unlock -- <input_file> [<output_path>] [-r] [-s]
```
```sh
cargo build --release -p fs-shapes-unlock
```

## fs-unpack

```
Usage: fs-unpack.exe <input> <output_path> [-s]

Extract .gar/.dlc archive

Positional Arguments:
  input             path to .gar/.dlc archive
  output_path       output path

Options:
  -s, --silent      silent mode
```

Extract GAR/DLC archives. Cross-platform, no external dependencies.
```sh
cargo run -p fs-unpack -- <archive> <output_path> [-s]
```
```sh
cargo build --release -p fs-unpack
```

## fs-xml-format

```
Usage: fs-xml-format.exe <input> [<output>] [-r] [-s] [-c <indent-char>] [-i <indent-size>] [-e]

Parse XML and output sane formatted XML.

Positional Arguments:
  input             path to input file/folder
  output            path to output file/folder (optional)

Options:
  -r, --recursive   recursive mode if folder input
  -s, --silent      suppress output
  -c, --indent-char indent character (space,tab)
  -i, --indent-size indent size
  -e, --disable-escape-characters
                    disable escape characters in attributes
```

```sh
cargo run -p fs-xml-format -- <input> [<output>] [-r] [-s] [-e] [-c <indent-char>] [-i <indent-size>]
```
```sh
cargo build --release -p fs-xml-format
```
