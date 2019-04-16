use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::spawn;
use std::time::Instant;

use osmpbfreader::OsmPbfReader;

use crate::parsers::{Area, AreaFactory};
use crate::prelude::*;

pub(crate) fn import_areas<AreaType, Factory>(
    path: &str,
    area_factory: &mut Factory,
) -> (
    HashMap<AreaId, AreaType>,
    HashMap<SegmentId, Segment>,
    HashMap<NodeId, Node>,
)
where
    AreaType: Area,
    Factory: AreaFactory<AreaType>,
{
    let file = File::open(&path).expect("Could not open input file! Exiting!");

    let mut reader = OsmPbfReader::new(file);

    let (inner_id_sender, inner_id_receiver) = channel();
    let inner_id_set_receiver = collect_ids(inner_id_receiver);

    let (outer_id_sender, outer_id_receiver) = channel();
    let outer_id_set_receiver = collect_ids(outer_id_receiver);

    let t1 = Instant::now();
    let mut areas = HashMap::new();
    areas.extend(
        reader
            .par_iter()
            .filter_map(std::result::Result::ok)
            .filter(osmpbfreader::objects::OsmObj::is_relation)
            .filter(|obj| area_factory.is_valid(obj.tags()))
            .filter_map(|obj| {
                if let Some(area) = obj.relation() {
                    return area_factory.to_area(area, &inner_id_sender, &outer_id_sender);
                }
                None
            })
            .map(|obj| (obj.get_id(), obj)),
    );
    let t2 = Instant::now();
    let areas = areas;

    let (node_id_sender, node_id_receiver) = channel();
    let node_id_set_receiver = collect_ids(node_id_receiver);

    drop(inner_id_sender);
    let inner_ids = inner_id_set_receiver
        .recv()
        .expect("Could not receive inner segment ids!");

    let t3 = Instant::now();
    let mut segments = import_ways(&inner_ids, &mut reader, &node_id_sender);
    drop(outer_id_sender);
    let outer_ids = outer_id_set_receiver
        .recv()
        .expect("Could not receive inner segment ids!");
    segments.append(&mut import_ways(&outer_ids, &mut reader, &node_id_sender));
    let t4 = Instant::now();

    drop(node_id_sender);
    let node_ids = node_id_set_receiver
        .recv()
        .expect("Could not receive node ids!");

    let t5 = Instant::now();
    let nodes = import_nodes(&node_ids, &mut reader);
    let t6 = Instant::now();

    let mut segmap = HashMap::new();
    segmap.extend(segments.into_iter().map(|seg| (seg.osmid, seg)));
    let mut nodemap = HashMap::new();
    nodemap.extend(nodes.into_iter().map(|node| (node.osmid, node)));

    println!(
        "Imported |   Number   | Time\n\
         ---------+------------+--------\n\
         Areas    |{:>11} | {:?}\n\
         Segments |{:>11} | {:?}\n\
         Nodes    |{:>11} | {:?}",
        areas.len(),
        t2.duration_since(t1),
        segmap.len(),
        t4.duration_since(t3),
        nodemap.len(),
        t6.duration_since(t5)
    );

    (areas, segmap, nodemap)
}

fn import_nodes(ids: &HashSet<NodeId>, osmreader: &mut OsmPbfReader<File>) -> Vec<Node> {
    osmreader.rewind().expect("Could not reset pbf file!");

    osmreader
        .par_iter()
        .filter_map(std::result::Result::ok)
        .filter(osmpbfreader::OsmObj::is_node)
        .filter_map(|obj| {
            if let osmpbfreader::OsmId::Node(id) = obj.id() {
                return Some((id, obj));
            }
            None
        })
        .filter(|obj| ids.contains(&obj.0))
        .filter_map(|obj| {
            if let Some(node) = obj.1.node() {
                return Some(Node {
                    osmid: node.id,
                    lat: node.decimicro_lat,
                    lon: node.decimicro_lon,
                });
            }
            None
        })
        .collect()
}

fn import_ways(
    ids: &HashSet<SegmentId>,
    osmreader: &mut OsmPbfReader<File>,
    node_id_sender: &Sender<NodeId>,
) -> Vec<Segment> {
    osmreader.rewind().expect("Could not reset pbf file!");

    osmreader
        .par_iter()
        .filter_map(std::result::Result::ok)
        .filter(osmpbfreader::OsmObj::is_way)
        .filter_map(|obj| {
            if let osmpbfreader::OsmId::Way(id) = obj.id() {
                return Some((id, obj));
            }
            None
        })
        .filter(|obj| ids.contains(&obj.0))
        .filter_map(|obj| {
            if let Some(way) = obj.1.way() {
                for nid in &way.nodes {
                    node_id_sender
                        .send(*nid)
                        .expect("Could not send node id when importing ways");
                }
                return Some(Segment {
                    osmid: way.id,
                    nodes: way.nodes.clone(),
                });
            }
            None
        })
        .collect()
}

fn collect_ids<IdType>(ids: Receiver<IdType>) -> Receiver<HashSet<IdType>>
where
    IdType: std::cmp::Eq + std::hash::Hash + std::marker::Send + 'static,
{
    let (send, recv) = channel();

    spawn(move || {
        let mut res = HashSet::new();
        for id in ids {
            res.insert(id);
        }
        send.send(res)
            .expect("Could not send node id set back to the receiver!");
    });

    recv
}
