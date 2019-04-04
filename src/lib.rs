use std::sync::mpsc::Sender;
use std::time::Instant;

use osmpbfreader;
use osmpbfreader::{Relation, Tags};

mod filter;
mod output;
mod parsers;

mod prelude {
    pub use crate::parsers::{AdminLevel, Latitude, Longitude};
    pub use crate::parsers::{AreaId, NodeId, SegmentId};
    pub use crate::parsers::{Node, Segment};

    pub use crate::parsers::Area;
    pub use crate::AdminArea;
}

use prelude::*;

pub struct AdminArea {
    pub osmid: AreaId,
    pub level: AdminLevel,
    pub name: String,

    pub inner: Vec<SegmentId>,
    pub outer: Vec<SegmentId>,
}

impl parsers::Area for AdminArea {
    fn get_id(&self) -> AreaId {
        self.osmid
    }

    fn get_inner(&self) -> &Vec<SegmentId> {
        &self.inner
    }

    fn get_outer(&self) -> &Vec<SegmentId> {
        &self.outer
    }
}

struct AdminAreaFactory {
    max_lvl: AdminLevel,
}

impl AdminAreaFactory {
    pub fn new(max_lvl: AdminLevel) -> Self {
        AdminAreaFactory { max_lvl }
    }
}

impl parsers::AreaFactory<AdminArea> for AdminAreaFactory {
    fn is_valid(&self, tags: &Tags) -> bool {
        if !tags.contains("type", "boundary") {
            return false;
        }
        if tags.get("name:en").is_none() && tags.get("name").is_none() {
            return false;
        }
        let admin_lvl = match tags.get("admin_level") {
            Some(val) => match val.parse::<u8>() {
                Ok(lvl) => lvl,
                Err(err) => {
                    eprintln!("Could not parse admin level value of area:\n{}", err);
                    return false;
                }
            },
            None => return false,
        };
        admin_lvl <= self.max_lvl
    }

    fn to_area(
        &self,
        rel: &Relation,
        inner_id_sender: &Sender<SegmentId>,
        outer_id_sender: &Sender<SegmentId>,
    ) -> Option<AdminArea> {
        assert!(self.is_valid(&rel.tags));

        let osmid = rel.id;
        // TODO: Improve error handling
        let level = match rel.tags.get("admin_level") {
            Some(val) => match val.parse::<u8>() {
                Ok(lvl) => lvl,
                Err(err) => {
                    eprintln!(
                        "Could not parse admin level value of area {}:\n{}",
                        osmid.0, err
                    );
                    return None;
                }
            },
            None => {
                eprintln!("Could not find admin_level tag for area {}", osmid.0);
                return None;
            }
        };
        if level > self.max_lvl {
            return None;
        }

        let name = match rel.tags.get("name:en") {
            Some(name) => name,
            None => {
                // if no english name could be found, take the general name ...
                match rel.tags.get("name") {
                    Some(name) => name,
                    None => return None,
                }
            }
        };

        let mut inner = Vec::new();
        let mut outer = Vec::new();

        for r in &rel.refs {
            match r.role.as_str() {
                "inner" => match r.member {
                    osmpbfreader::OsmId::Way(oid) => {
                        inner_id_sender.send(oid).unwrap();
                        inner.push(oid);
                    }
                    _ => {
                        eprintln!("Inner relation id is not a WayId in area {}", osmid.0);
                    }
                },
                "outer" => match r.member {
                    osmpbfreader::OsmId::Way(oid) => {
                        outer_id_sender.send(oid).unwrap();
                        outer.push(oid);
                    }
                    _ => eprintln!("Inner relation id is not a WayId in area {}", osmid.0),
                },
                _ => ()//eprintln!("Ignoring relation role {}", r.role),
            }
        }

        Some(AdminArea {
            osmid: osmid,
            level: level,
            name: name.clone(),

            inner: inner,
            outer: outer,
        })
    }
}

fn mercator(lat: &Latitude, lon: &Longitude) -> (Latitude, Longitude) {
    let _cntr_lat = 511633750;
    let cntr_lon = 0; //104476830;
    let scaling = 10000000_f64;

    // project according to: http://mathworld.wolfram.com/MercatorProjection.html
    let x = lon - cntr_lon;
    let lat_dec = (*lat as f64) / scaling;
    let lat_rad = lat_dec.to_radians();
    let y = lat_rad.sin().atanh().to_degrees();
    let lat_prj = (y * scaling) as Latitude;
    let lon_prj = x as Longitude;
    (lat_prj, lon_prj)
}

pub fn import_admin_areas(path: &String, max_lvl: AdminLevel) {
    let mut factory = AdminAreaFactory::new(max_lvl);

    let now = Instant::now();
    let (areas, segments, nodes) = parsers::import_areas(&path, &mut factory);
    let runtime = now.elapsed();

    println!(
        "Imported {} areas, {} segments and {} nodes in {:?}",
        areas.len(),
        segments.len(),
        nodes.len(),
        runtime
    );

    let complete_areas = filter::filter_complete(&areas, &segments, &nodes);
    println!("Complete areas are: {} many", complete_areas.len());

    output::to_file_with_deps(
        &"export.tmp".to_string(),
        &complete_areas,
        &areas,
        &segments,
        &nodes,
        &mercator,
    )
    .expect("Something went wrong when exporting to file!");
}

#[cfg(test)]
mod tests {}
