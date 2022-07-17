#![allow(clippy::too_many_arguments)]

use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf},
    process::exit,
};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{self, Parser};
use svg2polylines::{self, Polyline};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// The SVG file to load
    svgfile: PathBuf,

    /// Output path
    #[clap(long, help_heading = "DIRECTORIES")]
    outpath: PathBuf,

    /// Resulting LibrePCB package name
    #[clap(long, help_heading = "METADATA")]
    name: String,
    /// Resulting LibrePCB package description
    #[clap(long, default_value = "", help_heading = "METADATA")]
    description: String,
    /// Resulting LibrePCB package author
    #[clap(long, help_heading = "METADATA")]
    author: String,
    /// Resulting LibrePCB package version
    #[clap(long, default_value = "0.1.0", help_heading = "METADATA")]
    version: String,
    /// Resulting LibrePCB package keywords
    #[clap(long, default_value = "", help_heading = "METADATA")]
    keywords: String,

    /// Resulting LibrePCB package UUID [default: random]
    #[clap(long, help_heading = "UUIDS")]
    uuid_pkg: Option<String>,
    /// Resulting LibrePCB package category UUID
    #[clap(long, help_heading = "UUIDS")]
    uuid_pkgcat: String,

    /// Generate copper layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_copper: bool,
    /// Generate placement layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_placement: bool,
    /// Generate stop mask layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_stopmask: bool,

    /// Passed in by Inkscape, ignored, not currently supported
    #[clap(long, hide(true))]
    id: Option<Vec<String>>,
}

fn make_uuid() -> Uuid {
    Uuid::new_v4()
}

fn load_svg(path: &Path) -> Result<String> {
    Ok(read_to_string(path)?)
}

fn make_footprint(
    layer: &str,
    name: &str,
    description: &str,
    polylines: &[Polyline],
) -> Vec<String> {
    let mut lines = vec![];
    lines.push(format!(r#"(footprint {}"#, make_uuid()));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    for polyline in polylines {
        let closed = polyline[0] == polyline[polyline.len() - 1];
        let (width, fill) = match closed {
            true => ("0.0", "true"),
            false => ("0.2", "false"),
        };
        lines.push(format!(r#" (polygon "{}" (layer {})"#, make_uuid(), layer));
        lines.push(format!(r#"  (width {}) (fill {}) (grab_area true)"#, width, fill));
        for pair in polyline {
            lines.push(format!(
                r#"  (vertex (position {:.3} {:.3}) (angle 0.0))"#,
                pair.x, -pair.y
            ));
        }
        lines.push(r#" )"#.to_string());
    }
    lines.push(r#")"#.to_string());
    lines
}

fn make_package(
    uuid: &str,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    pkgcat: &str,
    footprints: &[Vec<String>],
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(r#"(librepcb_package {}"#, uuid));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(r#" (created {})"#, Utc::now().to_rfc3339().replace("+00:00", "Z")));
    lines.push(" (deprecated false)".to_string());
    lines.push(format!(r#" (category {})"#, pkgcat));
    for footprint in footprints {
        for line in footprint {
            lines.push(format!(" {}", line));
        }
    }
    lines.push(")".to_string());
    lines
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load and parse SVG
    let svg_string = load_svg(&args.svgfile).context("Could not read SVG file")?;
    let polylines = svg2polylines::parse(&svg_string, 0.15).expect("Could not parse SVG file");

    // Ensure that output library path exists
    let lib_path = match args.outpath.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Invalid output path: {}", e);
            exit(1);
        }
    };
    if !lib_path.exists() {
        eprintln!("Error: Output path {:?} does not exist", lib_path);
        exit(1);
    }
    if !lib_path.is_dir() {
        eprintln!("Error: Output path {:?} is not a directory", lib_path);
        exit(1);
    }

    // Generate footprints
    let mut footprints = vec![];
    if args.layer_copper {
        footprints.push(make_footprint("top_cu", "Top Copper", "", &polylines));
    }
    if args.layer_placement {
        footprints.push(make_footprint(
            "top_placement",
            "Top Placement",
            "",
            &polylines,
        ));
    }
    if args.layer_stopmask {
        footprints.push(make_footprint(
            "top_stop_mask",
            "Top Stop Mask",
            "",
            &polylines,
        ));
    }

    // Generate package
    let uuid_pkg = args.uuid_pkg.unwrap_or_else(|| make_uuid().to_string());
    let pkg = make_package(
        &uuid_pkg,
        &args.name,
        &args.description,
        &args.author,
        &args.keywords,
        &args.version,
        &args.uuid_pkgcat,
        &footprints,
    );

    // Write package to library
    let pkg_path = lib_path.join("pkg").join(&uuid_pkg);
    fs::create_dir_all(&pkg_path).unwrap();
    fs::write(pkg_path.join(".librepcb-pkg"), "0.1").unwrap();
    fs::write(pkg_path.join("package.lp"), pkg.join("\n")).unwrap();

    // Echo original SVG on stdout for compatibility with Inkscape.
    println!("{}", svg_string);

    Ok(())
}
