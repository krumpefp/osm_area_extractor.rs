use std::sync::mpsc::Sender;
use std::time::Instant;

use osmpbfreader;
use osmpbfreader::{Relation, Tags};

mod filter;
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
        return tags.contains("boundary", "administrative")
            && tags.get("name").is_some()
            && tags.get("admin_level").is_some();
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

        let name = match rel.tags.get("name") {
            Some(name) => name,
            None => {
                eprintln!("Could not get name of admin area {}", osmid.0);
                return None;
            }
        };

        let mut inner = Vec::new();
        let mut outer = Vec::new();

        for r in &rel.refs {
            match r.role.as_str() {
                "inner" => match r.member {
                    osmpbfreader::OsmId::Way(oid) => {
                        inner_id_sender.send(oid).unwrap();
                        inner.push(r.member);
                    }
                    _ => {
                        eprintln!("Inner relation id is not a WayId in area {}", osmid.0);
                    }
                },
                "outer" => match r.member {
                    osmpbfreader::OsmId::Way(oid) => {
                        outer_id_sender.send(oid).unwrap();
                        outer.push(r.member);
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

            inner: Vec::new(),
            outer: Vec::new(),
        })
    }
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
}

#[cfg(test)]
mod tests {}
