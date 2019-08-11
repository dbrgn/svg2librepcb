use std::f64::{INFINITY, NEG_INFINITY};
use std::fs::read_to_string;

use chrono::Utc;
use clap::{Arg, App};
use quick_error::quick_error;
use svg2polylines::{self, Polyline};
use uuid::Uuid;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
            cause(err)
            description(err.description())
        }
    }
}

fn make_uuid() -> Uuid {
    Uuid::new_v4()
}

fn load_svg(path: &str) -> Result<String, Error> {
    Ok(read_to_string(path)?)
}

fn make_footprint(layer: &str, name: &str, description: &str, polylines: &[Polyline]) -> Vec<String>{
    let mut lines = vec![];
    lines.push(format!(r#"(footprint {}"#, make_uuid()));
    lines.push(format!(r#" (name "{}")"#, name));
    lines.push(format!(r#" (description "{}")"#, description));
    for polyline in polylines {
        lines.push(format!(r#" (polygon "{}" (layer {})"#, make_uuid(), layer));
        lines.push(format!(r#"  (width 0.0) (fill true) (grab_area true)"#));
        for pair in polyline {
            lines.push(format!(r#"  (vertex (position {:.3} {:.3}) (angle 0.0))"#, pair.x, -pair.y));
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

fn center_object(polylines: &mut [Polyline]) {
    let (min_x, max_x, min_y, max_y) = polylines
        .iter()
        .flat_map(|polyline| polyline)
        .fold((INFINITY, NEG_INFINITY, INFINITY, NEG_INFINITY), |acc, pair| (
            f64::min(acc.0, pair.x),
            f64::max(acc.1, pair.x),
            f64::min(acc.2, pair.y),
            f64::max(acc.3, pair.y),
        ));
    let offset_x = -((max_x - min_x) / 2.0);
    let offset_y = -((max_y - min_y) / 2.0);
    for polyline in polylines {
        for pair in polyline {
            pair.x += offset_x;
            pair.y += offset_y;
        }
    }
}

fn main() {
    let param_svgfile = "SVGFILE";
    let param_name = "name";
    let param_description = "description";
    let param_author = "author";
    let param_pkg_uuid = "pkg_uuid";
    let param_pkgcat_uuid = "pkgcat_uuid";
    let param_keywords = "keywords";

    let matches = App::new(NAME)
        .version(VERSION)
        .author(AUTHORS)
        .about(DESCRIPTION)
        .arg(Arg::with_name(param_svgfile)
             .help("The SVG file to load")
             .required(true)
             .index(1))
        .arg(Arg::with_name(param_name)
             .short("n")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name(param_description)
             .short("d")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name(param_author)
             .short("a")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name(param_pkgcat_uuid)
             .short("c")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name(param_pkg_uuid)
             .short("u")
             .takes_value(true)
             .required(false))
        .arg(Arg::with_name(param_keywords)
             .short("k")
             .takes_value(true)
             .required(false))
        .get_matches();

    // Parse SVG, convert to polylines
    let svg_string = load_svg(matches.value_of(param_svgfile).unwrap())
        .expect("Could not read SVG file");
    let mut polylines = svg2polylines::parse(&svg_string)
        .expect("Could not parse SVG file");

    // Post-process polylines
    center_object(&mut polylines);

    let footprints = [
        make_footprint("top_placement", "Top Placement", "", &polylines),
        make_footprint("top_cu", "Top Copper", "", &polylines),
        make_footprint("top_stop_mask", "Top Stop Mask", "", &polylines),
    ];

    let pkg = make_package(
        matches.value_of(param_pkg_uuid),
        matches.value_of(param_name).unwrap(),
        matches.value_of(param_description).unwrap(),
        matches.value_of(param_author).unwrap(),
        matches.value_of(param_keywords).unwrap_or(""),
        "0.1.0",
        matches.value_of(param_pkgcat_uuid).unwrap(),
        &footprints,
    );

    for line in pkg {
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use svg2polylines::CoordinatePair;

    #[test]
    fn test_center_object() {
        let mut polylines = vec![
            vec![
                CoordinatePair::new(0.0, 0.0),
                CoordinatePair::new(2.0, 2.0),
            ],
            vec![
                CoordinatePair::new(2.0, 2.0),
                CoordinatePair::new(4.0, 4.0),
            ],
        ];
        center_object(&mut polylines);
        assert_eq!(polylines, vec![
            vec![
                CoordinatePair::new(-2.0, -2.0),
                CoordinatePair::new(0.0, 0.0),
            ],
            vec![
                CoordinatePair::new(0.0, 0.0),
                CoordinatePair::new(2.0, 2.0),
            ],
        ]);
    }
}
