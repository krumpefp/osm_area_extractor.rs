use std::env;

extern crate library;

fn main() {
    println!("Hello from Test");
    library::import_admin_areas(&"resources/pbfs/stuttgart-regbez-latest.osm.pbf".to_string());
}
