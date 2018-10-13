use crate::ecs::PrimitiveGeometryComponent;
use crate::geometry::boundingbox::BoundingBox;
use crate::types::*;
use specs::{Component, Entity, ReadStorage};

const CHILD_NODE_MAX_SIZE: usize = 8;

struct BvhNode {
    children: Vec<BvhNode>,
    center: Point3f,
    bbox: BoundingBox,
}

impl BvhNode {
    fn new(children: Vec<BvhNode>, center: Point3f, bbox: BoundingBox) -> BvhNode {
        BvhNode {
            children,
            center,
            bbox,
        }
    }

    fn new_from_cubes(
        storage: &ReadStorage<PrimitiveGeometryComponent>,
        entities: &[Entity],
    ) -> BvhNode {
        for entity in entities {
            if let Some(PrimitiveGeometryComponent::UnitCube(cube)) = storage.get(*entity) {
            } else {
                unimplemented!("Can only add unit cube to BVH node");
            }
        }
        BvhNode::new(
            vec![],
            Point3f::origin(),
            BoundingBox::new(Point3f::origin(), Point3f::origin()),
        )
    }
}

#[derive(Default, Debug)]
struct OctPartition {
    tfr: Vec<Point3f>,
    tfl: Vec<Point3f>,
    tbr: Vec<Point3f>,
    tbl: Vec<Point3f>,
    bfr: Vec<Point3f>,
    bfl: Vec<Point3f>,
    bbr: Vec<Point3f>,
    bbl: Vec<Point3f>,
}

impl PartialEq for OctPartition {
    fn eq(&self, other: &OctPartition) -> bool {
        self.tfr.eq(&other.tfr)
            && self.tfl.eq(&other.tfl)
            && self.tbr.eq(&other.tbr)
            && self.tbl.eq(&other.tbl)
            && self.bfr.eq(&other.bfr)
            && self.bfl.eq(&other.bfl)
            && self.bbr.eq(&other.bbr)
            && self.bbl.eq(&other.bbl)
    }
}

fn median<T: PartialOrd + Clone>(v: &mut [T]) -> T {
    if v.is_empty() {
        panic!("Cannot take median of empty slice");
    }
    let len = v.len();
    v.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    v[len / 2].clone()
}

fn partition_pts(pts: &[Point3f], origin: &Point3f) -> OctPartition {
    let mut partition = OctPartition::default();
    for pt in pts {
        let v = pt - origin;
        if v.x >= 0.0 {
            if v.y >= 0.0 {
                if v.z >= 0.0 {
                    partition.tfr.push(pt.clone());
                } else {
                    partition.tbr.push(pt.clone());
                }
            } else if v.z >= 0.0 {
                partition.bfr.push(pt.clone());
            } else {
                partition.bbr.push(pt.clone());
            }
        } else if v.y >= 0.0 {
            if v.z >= 0.0 {
                partition.tfl.push(pt.clone());
            } else {
                partition.tbl.push(pt.clone());
            }
        } else if v.z >= 0.0 {
            partition.bfl.push(pt.clone());
        } else {
            partition.bbl.push(pt.clone());
        }
    }
    partition
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_pts() {
        let pts = vec![
            Point3f::new(1.0, 1.0, 1.0),
            Point3f::new(1.0, 1.0, -1.0),
            Point3f::new(1.0, -1.0, 1.0),
            Point3f::new(1.0, -1.0, -1.0),
            Point3f::new(-1.0, 1.0, 1.0),
            Point3f::new(-1.0, 1.0, -1.0),
            Point3f::new(-1.0, -1.0, 1.0),
            Point3f::new(-1.0, -1.0, -1.0),
        ];
        assert_eq!(
            partition_pts(&pts, &Point3f::origin()),
            OctPartition {
                tfr: vec![Point3f::new(1.0, 1.0, 1.0),],
                tfl: vec![Point3f::new(-1.0, 1.0, 1.0)],
                tbr: vec![Point3f::new(1.0, 1.0, -1.0)],
                tbl: vec![Point3f::new(-1.0, 1.0, -1.0)],
                bfr: vec![Point3f::new(1.0, -1.0, 1.0)],
                bfl: vec![Point3f::new(-1.0, -1.0, 1.0)],
                bbr: vec![Point3f::new(1.0, -1.0, -1.0)],
                bbl: vec![Point3f::new(-1.0, -1.0, -1.0)],
            }
        );
    }
}
