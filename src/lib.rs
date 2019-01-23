use std::collections::HashMap;
use std::fs::File;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

use osmpbfreader;
use osmpbfreader::{Relation, Tags};

mod parsers;

pub use parsers::{AdminLevel, Latitude, Longitude};
pub use parsers::{AreaId, NodeId, SegmentId};
pub use parsers::{Node, Segment};

pub struct AdminArea {
    pub osmid: AreaId,
    pub level: AdminLevel,
    pub name: String,

    pub inner: Vec<SegmentId>,
    pub outer: Vec<SegmentId>,
}

struct AdminAreaFactory {
    areas: Vec<AdminArea>,
    segments: HashMap<SegmentId, Segment>,
    nodes: HashMap<NodeId, Node>,
}

impl AdminAreaFactory {
    fn new() -> Self {
        AdminAreaFactory {
            areas: Vec::new(),
            segments: HashMap::new(),
            nodes: HashMap::new(),
        }
    }
}

impl parsers::AreaFactory for AdminAreaFactory {
    type Area = AdminArea;

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

    fn set_segments(&mut self, segments: Vec<Segment>, nodes: Vec<Node>) {
        // TODO: remove clone
        self.segments
            .extend(segments.into_iter().map(|seg| (seg.osmid, seg)));
        self.nodes
            .extend(nodes.into_iter().map(|node| (node.osmid, node)));
    }
}

pub fn import_admin_areas(path: &String) {
    let mut factory = AdminAreaFactory::new();

    let now = Instant::now();
    parsers::import_areas(&path, &mut factory);
    let runtime = now.elapsed();

    println!(
        "Imported {} areas, {} segments and {} nodes in {:?}",
        factory.areas.len(),
        factory.segments.len(),
        factory.nodes.len(),
        runtime
    );
}

fn admin_obj(obj: &osmpbfreader::OsmObj) -> Option<&osmpbfreader::Relation> {
    // get relations with tags[boundary] == administrative
    let tags = obj.tags();
    if tags.contains("boundary", "administrative")
        && tags.get("name").is_some()
        && tags.get("admin_level").is_some()
    {
        return obj.relation();
    } else {
        return None;
    }
}

pub fn read_pbf(pbf_path: &String) {
    let f = File::open(pbf_path).unwrap();

    let mut pbf = osmpbfreader::OsmPbfReader::new(f);

    let (t_chan_inner, r_chan_inner) = channel();
    let (t_chan_outer, r_chan_outer) = channel();

    for obj in pbf.par_iter().map(Result::unwrap) {
        let rel = match admin_obj(&obj) {
            Some(rel) => rel,
            None => continue,
        };

        for r in &rel.refs {
            match r.role.as_str() {
                "inner" => t_chan_inner.send(r.member).unwrap(),
                "outer" => t_chan_outer.send(r.member).unwrap(),
                _ => eprintln!("Ignoring reference with role {}", r.role),
            }
        }
    }

    let inner_ids = vec![r_chan_inner];
    let outer_ids = vec![r_chan_outer];

    dbg!(inner_ids);
    dbg!(outer_ids);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn benchmark_stuttgart() {
        import_admin_areas(&"resources/pbfs/stuttgart-regbez-latest.osm.pbf".to_string());
    }
}
