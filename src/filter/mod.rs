extern crate rayon;
use rayon::prelude::*;

use std::collections::{HashMap, HashSet};

use crate::prelude::*;

pub fn filter_complete(
    areas: &HashMap<AreaId, AdminArea>,
    segments: &HashMap<SegmentId, Segment>,
    nodes: &HashMap<NodeId, Node>,
) -> Vec<AreaId> {
    let mut filtered_segs = HashSet::new();
    filtered_segs.par_extend(segments.into_par_iter().filter_map(|obj| {
        for n_id in &obj.1.nodes {
            if !nodes.contains_key(n_id) {
                return None;
            }
        }
        Some(*obj.0)
    }));
    let filtered_segs = filtered_segs;

    areas
        .into_par_iter()
        .filter_map(|obj| {
            for s_id in obj.1.get_inner().into_iter() {
                if !filtered_segs.contains(s_id) {
                    return None;
                }
            }
            for s_id in obj.1.get_outer().into_iter() {
                if !filtered_segs.contains(s_id) {
                    return None;
                }
            }
            Some(*obj.0)
        })
        .collect()
}
