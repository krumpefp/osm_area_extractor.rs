pub(crate) mod areaparser;

use osmpbfreader;
use osmpbfreader::{Relation, Tags};
use std::sync::mpsc::Sender;

pub type NodeId = osmpbfreader::NodeId;
pub type AreaId = osmpbfreader::RelationId;
pub type SegmentId = osmpbfreader::WayId;

pub type Latitude = i32;
pub type Longitude = i32;
pub type AdminLevel = u8;

pub(crate) use areaparser::import_areas;

#[derive(Debug)]
pub struct Node {
    pub osmid: NodeId,
    pub lat: Latitude,
    pub lon: Longitude,
}

#[derive(Debug)]
pub struct Segment {
    pub osmid: SegmentId,
    pub nodes: Vec<NodeId>,
}

pub trait Area {
    fn get_id(&self) -> AreaId;

    fn get_inner(&self) -> &Vec<SegmentId>;

    fn get_outer(&self) -> &Vec<SegmentId>;
}

pub(crate) trait AreaFactory<AreaType: Area> {
    ///
    /// Check for a given tag set of a relation if it is a valid area
    /// description
    ///
    fn is_valid(&self, tags: &Tags) -> bool;

    ///
    /// Create an area from the given relation.
    /// The <inner> and <outer>_segment_sender are channels to send the ids
    /// to. The corresponding segments and ids will be collected from the pbf
    /// if possible
    ///
    fn to_area(
        &self,
        rel: &Relation,
        inner_id_sender: &Sender<SegmentId>,
        outer_id_sender: &Sender<SegmentId>,
    ) -> Option<AreaType>;
}
