use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use ycd_animation_extractor::parse_ycd_animations;

#[derive(Serialize)]
struct Sidecar<'a> {
    dict: String,
    animations: &'a [String],
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

fn print_help() {
    println!(
        r#"ycd_animation_extractor — list clip names inside GTA V .ycd files

Usage:
  ycd_animation_extractor <folder> [options]

Options:
  -g [file]   Write merged dict → animation names to one JSON file.
              Default: <folder>/all_ycd_clips.json
  -p          Write one JSON file next to each .ycd (<name>.ycd.json)
  -h, --help  Show this help

The folder path must be the first argument. At least one of -g or -p is required.

Examples:
  ycd_animation_extractor "D:/mods" -g -p
  ycd_animation_extractor "D:/mods" -g "D:/out/all_ycd_clips.json" -p
"#
    );
}

fn dict_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn scan_folder(
    root_dir: &Path,
    write_combined: bool,
    write_sidecars: bool,
    combined_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    let mut combined: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut file_count = 0usize;
    let mut parse_errors = 0usize;

    for entry in WalkDir::new(root_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("ycd"))
            .unwrap_or(false)
        {
            continue;
        }

        file_count += 1;
        let bytes = fs::read(path)?;
        let dict = dict_name_from_path(path);
        let parsed = parse_ycd_animations(&bytes);

        if parsed.error.is_some() {
            parse_errors += 1;
        }

        if write_sidecars {
            let rel = path
                .strip_prefix(root_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let sidecar = Sidecar {
                dict: dict.clone(),
                animations: &parsed.animations,
                source: rel,
                error: parsed.error.as_deref(),
            };
            let out_path = path.with_extension("ycd.json");
            let json = serde_json::to_string_pretty(&sidecar)?;
            let mut f = File::create(out_path)?;
            f.write_all(json.as_bytes())?;
        }

        let entry = combined.entry(dict).or_insert_with(BTreeSet::new);
        for a in &parsed.animations {
            entry.insert(a.clone());
        }
    }

    if write_combined {
        let out_path = combined_path.unwrap_or_else(|| root_dir.join("all_ycd_clips.json"));
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let map_vec: BTreeMap<String, Vec<String>> = combined
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect();
        let json = serde_json::to_string_pretty(&map_vec)?;
        let mut f = File::create(&out_path)?;
        f.write_all(json.as_bytes())?;

        println!("Wrote {} dict(s) to {}", map_vec.len(), out_path.display());
    }

    if write_sidecars {
        println!("Wrote per-file JSON next to each .ycd ({file_count} file(s)).");
    }
    if parse_errors > 0 {
        eprintln!(
            "{parse_errors} file(s) had parse errors (see per-file JSON error fields if -p was used)."
        );
    }
    println!("Processed {file_count} .ycd file(s).");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        print_help();
        return Ok(());
    }

    if args[0].starts_with('-') {
        eprintln!("Error: the folder path must be the first argument (before options).\n");
        print_help();
        std::process::exit(1);
    }

    let root_dir = PathBuf::from(&args[0]);
    let rest = &args[1..];

    let mut write_combined = false;
    let mut write_sidecars = false;
    let mut combined_path: Option<PathBuf> = None;

    let mut i = 0usize;
    while i < rest.len() {
        match rest[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-p" => {
                write_sidecars = true;
                i += 1;
            }
            "-g" => {
                write_combined = true;
                if i + 1 < rest.len() && !rest[i + 1].starts_with('-') {
                    combined_path = Some(PathBuf::from(&rest[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            other => {
                eprintln!("Unknown option: {other}");
                print_help();
                std::process::exit(1);
            }
        }
    }

    if !write_combined && !write_sidecars {
        eprintln!("Error: specify -g and/or -p.\n");
        print_help();
        std::process::exit(1);
    }

    let combined = if write_combined {
        Some(
            combined_path
                .unwrap_or_else(|| root_dir.join("all_ycd_clips.json")),
        )
    } else {
        None
    };

    scan_folder(
        &root_dir,
        write_combined,
        write_sidecars,
        combined,
    )
}
