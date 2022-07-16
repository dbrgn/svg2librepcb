use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
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
    /// Resulting LibrePCB package name
    #[clap(short)]
    name: String,
    /// Resulting LibrePCB package description
    #[clap(short)]
    description: String,
    /// Resulting LibrePCB package author
    #[clap(short)]
    author: String,
    /// Resulting LibrePCB package category UUID
    #[clap(short = 'c')]
    pkgcat_uuid: String,
    /// Resulting LibrePCB package UUID (optional)
    #[clap(short = 'u')]
    pkg_uuid: Option<String>,
    /// Resulting LibrePCB package keywords (optional)
    #[clap(short)]
    keywords: Option<String>,
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
        lines.push(format!(r#" (polygon "{}" (layer {})"#, make_uuid(), layer));
        lines.push(format!(r#"  (width 0.0) (fill true) (grab_area true)"#));
        for pair in polyline {
            lines.push(format!(
                r#"  (vertex (position {:.3} {:.3}) (angle 0.0))"#,
                pair.x, -pair.y
            ));
        }
        lines.push(format!(r#" )"#));
    }
    lines.push(format!(r#")"#));
    lines
}

fn make_package(
    uuid: Option<&str>,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    pkgcat: &str,
    footprints: &[Vec<String>],
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(
        r#"(librepcb_package {}"#,
        uuid.map(|u| u.to_string())
            .unwrap_or_else(|| make_uuid().to_string()),
    ));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(r#" (created {})"#, Utc::now().to_rfc3339()));
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

    let svg_string = load_svg(&args.svgfile).context("Could not read SVG file")?;
    let polylines = svg2polylines::parse(&svg_string, 0.15).expect("Could not parse SVG file");

    let footprints = [
        make_footprint("top_placement", "Top Placement", "", &polylines),
        make_footprint("top_cu", "Top Copper", "", &polylines),
        make_footprint("top_stop_mask", "Top Stop Mask", "", &polylines),
    ];

    let pkg = make_package(
        args.pkg_uuid.as_deref(),
        &args.name,
        &args.description,
        &args.author,
        args.keywords.as_deref().unwrap_or(""),
        "0.1.0",
        &args.pkgcat_uuid,
        &footprints,
    );

    for line in pkg {
        println!("{}", line);
    }

    Ok(())
}
