#![allow(clippy::too_many_arguments)]
#![allow(clippy::useless_format)]

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
    /// Resulting LibrePCB symbol UUID [default: random]
    #[clap(long, help_heading = "UUIDS")]
    uuid_sym: Option<String>,
    /// Resulting LibrePCB component UUID [default: random]
    #[clap(long, help_heading = "UUIDS")]
    uuid_cmp: Option<String>,
    /// Resulting LibrePCB device UUID [default: random]
    #[clap(long, help_heading = "UUIDS")]
    uuid_dev: Option<String>,
    /// Resulting LibrePCB package category UUID
    #[clap(long, help_heading = "UUIDS")]
    uuid_pkgcat: Option<String>,
    /// Resulting LibrePCB symbol category UUID
    #[clap(long, help_heading = "UUIDS")]
    uuid_cmpcat: Option<String>,

    /// Generate copper layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_copper: bool,
    /// Generate placement layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_placement: bool,
    /// Generate stop mask layer
    #[clap(long, default_value = "true", help_heading = "LAYERS")]
    layer_stopmask: bool,

    /// Flattening tolerance
    #[clap(long, default_value = "0.15", help_heading = "PARAMETERS")]
    flattening_tolerance: f64,
    /// Align the centerpoint
    #[clap(long, value_enum, default_value = "none", help_heading = "PARAMETERS")]
    align: Align,

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

#[derive(PartialEq, Eq, Copy, Clone, Debug, clap::ValueEnum)]
enum Align {
    None,
    Center,
    TopLeft,
    BottomLeft,
}

#[derive(Default)]
struct Bounds {
    y_min: f64,
    y_max: f64,
}

struct Polygon {
    /// Polygon lines
    lines: Vec<String>,
    /// Transformed bounds (in the LibrePCB coordinate system)
    transformed_bounds: Bounds,
}

/// Format a float according to LibrePCB normalization rules.
fn format_float(val: f64) -> String {
    if val == -0.0 {
        // Returns true for 0.0 too, but that doesn't matter
        return "0.0".to_string();
    }
    let formatted = format!("{:.3}", val);
    if formatted.ends_with('0') {
        // 1 trailing zero
        if formatted.chars().rev().nth(1).unwrap() == '0' {
            // 2 trailing zeroes
            return formatted.strip_suffix("00").unwrap().to_string();
        }
        return formatted.strip_suffix('0').unwrap().to_string();
    }
    formatted
}

fn make_polygon(layer: &str, align: Align, polylines: &[Polyline]) -> Polygon {
    let mut lines = vec![];
    if polylines.is_empty() {
        return Polygon {
            lines,
            transformed_bounds: Bounds::default(),
        };
    }

    // Note: In SVG, the top left point is (0, 0). The y-axis expands
    //       downwards. In LibrePCB, the Y axis is the other way around, and
    //       expands upwards.

    // First, find bounds to allow centering
    let first_pair = polylines[0][0];
    let (mut x_min, mut x_max, mut y_min, mut y_max) =
        (first_pair.x, first_pair.x, first_pair.y, first_pair.y);
    for polyline in polylines {
        for pair in polyline {
            x_min = pair.x.min(x_min);
            x_max = pair.x.max(x_max);
            y_min = pair.y.min(y_min);
            y_max = pair.y.max(y_max);
        }
    }

    // Calculate offset (still in SVG coordinate mode)
    let (dx, dy) = match align {
        Align::None => (0.0, 0.0),
        Align::Center => {
            let halfwidth = (x_max - x_min) / 2.0;
            let halfheight = (y_max - y_min) / 2.0;
            (-x_min - halfwidth, -y_min - halfheight)
        }
        Align::TopLeft => (-x_min, -y_min),
        Align::BottomLeft => (-x_min, -y_max),
    };

    // Then generate vertices
    for polyline in polylines {
        let closed = polyline[0] == polyline[polyline.len() - 1];
        let (width, fill) = match closed {
            true => ("0.0", "true"),
            false => ("0.2", "false"),
        };
        lines.push(format!(r#" (polygon "{}" (layer {})"#, make_uuid(), layer));
        lines.push(format!(
            r#"  (width {0}) (fill {1}) (grab_area {1})"#,
            width, fill
        ));
        for pair in polyline {
            lines.push(format!(
                r#"  (vertex (position {:.3} {:.3}) (angle 0.0))"#,
                pair.x + dx,
                -(pair.y + dy) // Invert axis
            ));
        }
        lines.push(r#" )"#.to_string());
    }

    Polygon {
        lines,
        transformed_bounds: Bounds {
            y_min: y_min + dy,
            y_max: y_max + dy,
        },
    }
}

fn make_footprint(
    layer: &str,
    name: &str,
    description: &str,
    align: Align,
    polylines: &[Polyline],
) -> Vec<String> {
    let mut lines = vec![];
    lines.push(format!(r#"(footprint {}"#, make_uuid()));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    if !polylines.is_empty() {
        lines.extend_from_slice(&make_polygon(layer, align, polylines).lines);
    }
    lines.push(r#")"#.to_string());
    lines
}

fn make_symbol(
    uuid: &str,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    uuid_cmpcat: Option<&str>,
    polylines: &[Polyline],
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(r#"(librepcb_symbol {}"#, uuid));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(
        r#" (created {})"#,
        Utc::now().to_rfc3339().replace("+00:00", "Z")
    ));
    lines.push(" (deprecated false)".to_string());
    if let Some(uuid) = uuid_cmpcat {
        lines.push(format!(r#" (category {})"#, uuid));
    }

    // Polygon
    let polygon = make_polygon("sym_outlines", Align::Center, polylines);
    lines.extend_from_slice(&polygon.lines);

    // Label: Value
    lines.push(format!(
        r#" (text {} (layer sym_values) (value "{{{{VALUE}}}}")"#,
        make_uuid()
    ));
    lines.push(format!(
        r#"  (align center top) (height 2.5) (position 0.0 {}) (rotation 0.0)"#,
        format_float(polygon.transformed_bounds.y_min - 1.27)
    ));
    lines.push(" )".to_string());

    // Label: Name
    lines.push(format!(
        r#" (text {} (layer sym_names) (value "{{{{NAME}}}}")"#,
        make_uuid()
    ));
    lines.push(format!(
        r#"  (align center bottom) (height 2.5) (position 0.0 {}) (rotation 0.0)"#,
        format_float(polygon.transformed_bounds.y_max + 1.27)
    ));
    lines.push(" )".to_string());

    lines.push(")".to_string());
    lines
}

fn make_component(
    uuid: &str,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    uuid_sym: &str,
    uuid_cmpcat: Option<&str>,
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(r#"(librepcb_component {}"#, uuid));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(
        r#" (created {})"#,
        Utc::now().to_rfc3339().replace("+00:00", "Z")
    ));
    lines.push(r#" (deprecated false)"#.to_string());
    if let Some(uuid) = uuid_cmpcat {
        lines.push(format!(r#" (category {})"#, uuid));
    }
    lines.push(format!(r#" (schematic_only false)"#));
    lines.push(format!(r#" (default_value "")"#));
    lines.push(format!(r#" (prefix "")"#));
    lines.push(format!(r#" (variant {} (norm "")"#, make_uuid()));
    lines.push(format!(r#"  (name "default")"#));
    lines.push(format!(r#"  (description "")"#));
    lines.push(format!(r#"  (gate {}"#, make_uuid()));
    lines.push(format!(r#"   (symbol {})"#, uuid_sym));
    lines.push(format!(
        r#"   (position 0.0 0.0) (rotation 0.0) (required true) (suffix "")"#
    ));
    lines.push(format!(r#"  )"#));
    lines.push(format!(r#" )"#));
    lines.push(format!(")"));
    lines
}

fn make_package(
    uuid: &str,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    uuid_pkgcat: Option<&str>,
    footprints: &[Vec<String>],
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(r#"(librepcb_package {}"#, uuid));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(
        r#" (created {})"#,
        Utc::now().to_rfc3339().replace("+00:00", "Z")
    ));
    lines.push(" (deprecated false)".to_string());
    if let Some(uuid) = uuid_pkgcat {
        lines.push(format!(r#" (category {})"#, uuid));
    }
    for footprint in footprints {
        for line in footprint {
            lines.push(format!(" {}", line));
        }
    }
    lines.push(")".to_string());
    lines
}

fn make_device(
    uuid: &str,
    name: &str,
    description: &str,
    keywords: &str,
    author: &str,
    version: &str,
    uuid_cmp: &str,
    uuid_pkg: &str,
    uuid_cmpcat: Option<&str>,
) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    lines.push(format!(r#"(librepcb_device {}"#, uuid));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    lines.push(format!(r#" (keywords "{}")"#, keywords));
    lines.push(format!(r#" (author "{}")"#, author));
    lines.push(format!(r#" (version "{}")"#, version));
    lines.push(format!(
        r#" (created {})"#,
        Utc::now().to_rfc3339().replace("+00:00", "Z")
    ));
    lines.push(r#" (deprecated false)"#.to_string());
    if let Some(uuid) = uuid_cmpcat {
        lines.push(format!(r#" (category {})"#, uuid));
    }
    lines.push(format!(r#" (component {})"#, uuid_cmp));
    lines.push(format!(r#" (package {})"#, uuid_pkg));
    lines.push(format!(")"));
    lines
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load and parse SVG
    let svg_string = load_svg(&args.svgfile).context("Could not read SVG file")?;
    let polylines = svg2polylines::parse(&svg_string, args.flattening_tolerance)
        .expect("Could not parse SVG file");

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
        footprints.push(make_footprint(
            "top_cu",
            "Top Copper",
            "",
            args.align,
            &polylines,
        ));
    }
    if args.layer_placement {
        footprints.push(make_footprint(
            "top_placement",
            "Top Placement",
            "",
            args.align,
            &polylines,
        ));
    }
    if args.layer_stopmask {
        footprints.push(make_footprint(
            "top_stop_mask",
            "Top Stop Mask",
            "",
            args.align,
            &polylines,
        ));
    }

    // Generate symbol
    let uuid_sym = args.uuid_sym.unwrap_or_else(|| make_uuid().to_string());
    let sym = make_symbol(
        &uuid_sym,
        &args.name,
        &args.description,
        &args.author,
        &args.keywords,
        &args.version,
        args.uuid_cmpcat.as_deref(),
        &polylines,
    );

    // Generate component
    let uuid_cmp = args.uuid_cmp.unwrap_or_else(|| make_uuid().to_string());
    let cmp = make_component(
        &uuid_cmp,
        &args.name,
        &args.description,
        &args.author,
        &args.keywords,
        &args.version,
        &uuid_sym,
        args.uuid_cmpcat.as_deref(),
    );

    // Generate package
    let uuid_pkg = args.uuid_pkg.unwrap_or_else(|| make_uuid().to_string());
    let pkg = make_package(
        &uuid_pkg,
        &args.name,
        &args.description,
        &args.author,
        &args.keywords,
        &args.version,
        args.uuid_pkgcat.as_deref(),
        &footprints,
    );

    // Generate device
    let uuid_dev = args.uuid_dev.unwrap_or_else(|| make_uuid().to_string());
    let dev = make_device(
        &uuid_dev,
        &args.name,
        &args.description,
        &args.author,
        &args.keywords,
        &args.version,
        &uuid_cmp,
        &uuid_pkg,
        args.uuid_cmpcat.as_deref(),
    );

    // Write files to library
    let sym_path = lib_path.join("sym").join(&uuid_sym);
    let cmp_path = lib_path.join("cmp").join(&uuid_cmp);
    let pkg_path = lib_path.join("pkg").join(&uuid_pkg);
    let dev_path = lib_path.join("dev").join(&uuid_dev);
    fs::create_dir_all(&sym_path).unwrap();
    fs::create_dir_all(&cmp_path).unwrap();
    fs::create_dir_all(&pkg_path).unwrap();
    fs::create_dir_all(&dev_path).unwrap();
    fs::write(sym_path.join(".librepcb-sym"), "0.1").unwrap();
    fs::write(cmp_path.join(".librepcb-cmp"), "0.1").unwrap();
    fs::write(pkg_path.join(".librepcb-pkg"), "0.1").unwrap();
    fs::write(dev_path.join(".librepcb-dev"), "0.1").unwrap();
    fs::write(sym_path.join("symbol.lp"), sym.join("\n")).unwrap();
    fs::write(cmp_path.join("component.lp"), cmp.join("\n")).unwrap();
    fs::write(pkg_path.join("package.lp"), pkg.join("\n")).unwrap();
    fs::write(dev_path.join("device.lp"), dev.join("\n")).unwrap();

    // Echo original SVG on stdout for compatibility with Inkscape.
    println!("{}", svg_string);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_float() {
        let cases = [
            (3.14456, "3.145"),
            (-7.0, "-7.0"),
            (0.4, "0.4"),
            (-0.0, "0.0"),
        ];
        for case in cases {
            assert_eq!(format_float(case.0), case.1);
        }
    }
}
