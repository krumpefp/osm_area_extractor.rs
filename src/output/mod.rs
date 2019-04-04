extern crate rayon;

use crate::prelude::*;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufWriter, Write};

pub fn to_file_with_deps(
    path: &String,
    area_ids: &Vec<AreaId>,
    areas: &HashMap<AreaId, AdminArea>,
    segs: &HashMap<SegmentId, Segment>,
    nodes: &HashMap<NodeId, Node>,
    projection: &Fn(&Latitude, &Longitude) -> (Latitude, Longitude),
) -> Result<(), ()> {
    let mut writer =
        BufWriter::new(File::create(path.as_str()).expect("Could not open output file!"));

    // collect the nodes and segment ids we need to write
    let (node_ids, seg_ids) =
        nodes_segs_to_write(area_ids, areas, segs).expect("Could not find some required segments!");

    // write the nodes
    writeln!(&mut writer, "Nodecount:{}", node_ids.len()).expect("Could not write to output file!");
    for n_id in node_ids {
        let node = &nodes[&n_id];
        let prj = projection(&node.lat, &node.lon);
        writeln!(&mut writer, "{}:{},{};", node.osmid.0, prj.0, prj.1)
            .expect("Could not write to output file!");
    }

    // write the segments
    writeln!(&mut writer, "Segmentcount:{}", seg_ids.len())
        .expect("Could not write to output file!");
    for s_id in seg_ids {
        let seg = &segs[&s_id];
        let node_list: Vec<String> = seg.nodes.iter().map(|n_id| n_id.0.to_string()).collect();
        let node_str = node_list.join(",");
        writeln!(&mut writer, "{},0:{};", seg.osmid.0, node_str)
            .expect("Could not write to output file!");
    }

    // write the areas
    writeln!(&mut writer, "Areacount:{}", area_ids.len()).expect("Could not write to output file!");
    for a_id in area_ids {
        let area = &areas[&a_id];
        writeln!(
            &mut writer,
            "{},{},{}:{},{},0",
            area.osmid.0,
            area.level,
            area.name,
            area.outer.len(),
            area.inner.len()
        )
        .expect("Could not write to output file!");
        if area.outer.len() > 0 {
            let seg_list: Vec<String> = area.outer.iter().map(|s_id| s_id.0.to_string()).collect();
            let seg_str = seg_list.join(",");
            writeln!(&mut writer, "{}", seg_str).expect("Could not write to output file!");
        }
        if area.inner.len() > 0 {
            let seg_list: Vec<String> = area.inner.iter().map(|s_id| s_id.0.to_string()).collect();
            let seg_str = seg_list.join(",");
            writeln!(&mut writer, "{}", seg_str).expect("Could not write to output file!");
        }
    }

    Ok(())
}

fn nodes_segs_to_write(
    area_ids: &Vec<AreaId>,
    areas: &HashMap<AreaId, AdminArea>,
    segs: &HashMap<SegmentId, Segment>,
) -> Result<(HashSet<NodeId>, HashSet<SegmentId>), String> {
    let mut seg_ids = HashSet::new();
    for a_id in area_ids {
        let a = &areas[a_id];
        seg_ids.extend(a.inner.iter());
        seg_ids.extend(a.outer.iter());
    }

    let mut node_ids: HashSet<NodeId> = HashSet::new();
    for s_id in &seg_ids {
        let s = &segs[&s_id];
        node_ids.extend(s.nodes.iter());
    }

    Ok((node_ids, seg_ids))
}
