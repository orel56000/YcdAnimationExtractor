# YcdAnimationExtractor

CLI tool that lists clip / animation names inside **GTA V** `.ycd` (clip dictionary) files.

## Compile

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

From the repository root:

```bash
cargo build --release
```

- **Debug build** (faster compile, slower binary): `cargo build`  
- **Release build** (what you usually want): `cargo build --release`

Output location:

| Platform | Binary path |
|----------|-------------|
| Windows | `target\release\ycd_animation_extractor.exe` |
| Linux / macOS | `target/release/ycd_animation_extractor` |

You can copy that file anywhere on your `PATH`, or run it with a full path as shown below.

## Recommended use

Most of the time you only need a single merged JSON next to your mod folder (default output name: `all_ycd_clips.json` inside that folder):

**Windows**

```powershell
ycd_animation_extractor.exe "D:\mods\animations" -g
```

**Linux**

```bash
./ycd_animation_extractor "/home/you/mods/animations" -g
```

Use `-g "C:\path\to\out.json"` (Windows) or `-g /path/to/out.json` (Linux) if you want a specific output file.

## Usage

```text
ycd_animation_extractor <folder> [options]

Options:
  -g [file]   Write merged dict → animation names to one JSON file.
              Default: <folder>/all_ycd_clips.json
  -p          Write one JSON file next to each .ycd (<name>.ycd.json)
  -h, --help  Show help
```

The folder path must be the first argument. At least one of `-g` or `-p` is required.

### Examples

**Windows**

```powershell
ycd_animation_extractor.exe "D:\mods" -g -p
ycd_animation_extractor.exe "D:\mods" -g "D:\out\all_ycd_clips.json" -p
```

**Linux**

```bash
./ycd_animation_extractor "/home/you/mods" -g -p
./ycd_animation_extractor "/home/you/mods" -g "/home/you/out/all_ycd_clips.json" -p
```

## Library

The crate exposes `parse_ycd_animations` and related helpers for use from other Rust code. See `src/lib.rs`.

## Credits

The algorithm and file layout for reading `.ycd` (clip dictionary) data are derived from [CodeWalker](https://github.com/dexyfex/CodeWalker).

## License

MIT. See [LICENSE](LICENSE).
