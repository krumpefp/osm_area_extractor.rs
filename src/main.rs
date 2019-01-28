extern crate clap;

use clap::{value_t, App, Arg};

extern crate library;

fn main() {
    let params = create_cli_interface().get_matches();

    let path = params
        .value_of("input")
        .expect("Could not find parameter value for input");
    let max_lvl = value_t!(params.value_of("max_admin_lvl"), u8)
        .expect("Could not find parameter value for maximum admin level");
    library::import_admin_areas(&path.to_string(), max_lvl);
}

fn create_cli_interface<'a>() -> App<'a, 'a> {
    App::new("OpenStreetMap Area Extractor - Rust")
        .version("0.1")
        .author("Filip Krumpe <filip.krumpe@fmi.uni-stuttgart.de")
        .about("Import Area Features from pbf files")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("PBF FILE")
                .help("Input pbf file")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("max_admin_lvl")
            .short("mx")
            .long("max_lvl")
            .value_name("UNSIGNED")
            .help("Maximum administrative level to extract according to https://wiki.openstreetmap.org/wiki/Tag%3aboundary=administrative. Default = 4 (region level 'state-district')")
            .takes_value(true)
            .default_value("4")
        )
}
