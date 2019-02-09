use crate::{
    ecs::{entity::Entity, AABBComponent, PrimitiveGeometryComponent, TransformComponent},
    geometry::{ray::Ray, AABB},
    types::prelude::*,
};
use num_traits::identities::Zero;
use specs::ReadStorage;

const CHILD_NODE_MAX_SIZE: usize = 8;

#[derive(Debug, Clone)]
struct Node {
    center: Point3f,
    bbox: AABB,
    children: Option<NodeOctPartition>,
    entities: Vec<Entity>,
}

impl Node {
    // fn _new_from_cubes(
    //     entities: &[Entity],
    //     transform_storage: &ReadStorage<TransformComponent>,
    //     bbox_storage: &ReadStorage<AABBComponent>,
    //     child_node_max_size: usize,
    // ) -> Option<Node> {
    //     if entities.is_empty() {
    //         return None;
    //     }
    //     let centers: Vec<Point3f> = entities
    //         .iter()
    //         .map(|ety| {
    //             if let Some(TransformComponent(transform)) = transform_storage.get(ety.entity) {
    //                 transform.translation()
    //             } else {
    //                 unreachable!()
    //             }
    //         })
    //         .collect();
    //     let origin_x = median(&mut centers.iter().map(|&p| p.x).collect::<Vec<f32>>());
    //     let origin_y = median(&mut centers.iter().map(|&p| p.y).collect::<Vec<f32>>());
    //     let origin_z = median(&mut centers.iter().map(|&p| p.z).collect::<Vec<f32>>());
    //     let origin = Point3f::new(origin_x, origin_y, origin_z);

    //     let bboxes: Vec<AABB> = entities
    //         .iter()
    //         .map(|ety| *ety.bounding_box(bbox_storage))
    //         .collect();

    //     if entities.len() > child_node_max_size {
    //         let partition = partition_pts(transform_storage, entities, &origin);
    //         let tfr = Box::new(Node::_new_from_cubes(
    //             &partition.tfr,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let tfl = Box::new(Node::_new_from_cubes(
    //             &partition.tfl,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let tbr = Box::new(Node::_new_from_cubes(
    //             &partition.tbr,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let tbl = Box::new(Node::_new_from_cubes(
    //             &partition.tbl,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let bfr = Box::new(Node::_new_from_cubes(
    //             &partition.bfr,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let bfl = Box::new(Node::_new_from_cubes(
    //             &partition.bfl,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let bbr = Box::new(Node::_new_from_cubes(
    //             &partition.bbr,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         let bbl = Box::new(Node::_new_from_cubes(
    //             &partition.bbl,
    //             transform_storage,
    //             bbox_storage,
    //             child_node_max_size,
    //         ));
    //         Some(Node::new(
    //             AABB::merge_bboxes(&bboxes),
    //             NodeType::Internal(OctPartition {
    //                 tfr,
    //                 tfl,
    //                 tbr,
    //                 tbl,
    //                 bfr,
    //                 bfl,
    //                 bbr,
    //                 bbl,
    //             }),
    //         ))
    //     } else {
    //         Some(Node::new(
    //             AABB::merge_bboxes(&bboxes),
    //             NodeType::Leaf(entities.to_vec()),
    //         ))
    //     }
    // }

    // pub fn new_from_cubes(
    //     entities: &[Entity],
    //     transform_storage: &ReadStorage<TransformComponent>,
    //     bbox_storage: &ReadStorage<AABBComponent>,
    // ) -> Option<Node> {
    //     Node::_new_from_cubes(
    //         entities,
    //         transform_storage,
    //         bbox_storage,
    //         CHILD_NODE_MAX_SIZE,
    //     )
    // }

    // pub fn intersected_cube(
    //     &self,
    //     ray: &Ray,
    //     storage: &ReadStorage<AABBComponent>,
    // ) -> Option<Entity> {
    //     if ray.intersect_bbox(&self.bbox).is_none() {
    //         return None;
    //     }
    //     match &self.ty {
    //         NodeType::Leaf(entities) => ray
    //             .intersect_entities(entities, storage)
    //             .map(|(i, _)| entities[i]),
    //         NodeType::Internal(oct_partition) => {
    //             let mut entities = vec![];
    //             if let Some(bvh_node) = oct_partition.tfr.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.tfl.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.tbr.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.tbl.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.bfr.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.bfl.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.bbr.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }
    //             if let Some(bvh_node) = oct_partition.bbl.as_ref() {
    //                 if let Some(ety) = bvh_node.intersected_cube(ray, storage) {
    //                     entities.push(ety);
    //                 }
    //             }

    //             ray.intersect_entities(&entities, storage)
    //                 .map(|(i, p)| entities[i])
    //         }
    //     }
    // }

    // pub fn add(
    //     &mut self,
    //     entity: Entity,
    //     transform_storage: &ReadStorage<TransformComponent>,
    //     bbox_storage: &ReadStorage<AABBComponent>,
    // ) {

    // }
}

#[derive(Clone, Debug)]
struct NodeOctPartition {
    tfr: Box<Option<Node>>,
    tfl: Box<Option<Node>>,
    tbr: Box<Option<Node>>,
    tbl: Box<Option<Node>>,
    bfr: Box<Option<Node>>,
    bfl: Box<Option<Node>>,
    bbr: Box<Option<Node>>,
    bbl: Box<Option<Node>>,
}

#[derive(Default, Debug)]
struct OctPartition {
    tfr: Vec<Entity>,
    tfl: Vec<Entity>,
    tbr: Vec<Entity>,
    tbl: Vec<Entity>,
    bfr: Vec<Entity>,
    bfl: Vec<Entity>,
    bbr: Vec<Entity>,
    bbl: Vec<Entity>,
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

impl Eq for OctPartition {}

// fn partition_pts(
//     storage: &ReadStorage<TransformComponent>,
//     entities: &[Entity],
//     origin: &Point3f,
// ) -> OctPartition {
//     let mut partition = OctPartition::default();
//     for entity in entities {
//         let v = entity.position(storage) - origin;
//         if v.x >= 0.0 {
//             if v.y >= 0.0 {
//                 if v.z >= 0.0 {
//                     partition.tfr.push(entity.clone());
//                 } else {
//                     partition.tbr.push(entity.clone());
//                 }
//             } else if v.z >= 0.0 {
//                 partition.bfr.push(entity.clone());
//             } else {
//                 partition.bbr.push(entity.clone());
//             }
//         } else if v.y >= 0.0 {
//             if v.z >= 0.0 {
//                 partition.tfl.push(entity.clone());
//             } else {
//                 partition.tbl.push(entity.clone());
//             }
//         } else if v.z >= 0.0 {
//             partition.bfl.push(entity.clone());
//         } else {
//             partition.bbl.push(entity.clone());
//         }
//     }
//     partition
// }

// fn partition_entities(entities: &[Entity], point: &Point3f) -> (Vec<Entity>, OctPartition) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::pt3f::Point3fExt;
    use alga::general::SubsetOf;
    use specs::World;

    #[test]
    fn test_partition_pts() {
        // let mut world = World::new();
        // world.register::<TransformComponent>();

        // let entities = vec![
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, 1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, -1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, 1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, -1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, 1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, -1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, 1.0)),
        //     ),
        //     Entity::new_with_transform_w(
        //         &world,
        //         Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, -1.0)),
        //     ),
        // ];

        // let partition = partition_pts(&world.read_storage(), &entities, &Point3f::origin());
        // let storage = world.read_storage();
        // assert_eq!(
        //     *partition.tfr[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, 1.0))
        // );
        // assert_eq!(
        //     *partition.tfl[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, 1.0))
        // );
        // assert_eq!(
        //     *partition.tbr[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, -1.0))
        // );
        // assert_eq!(
        //     *partition.tbl[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, -1.0))
        // );
        // assert_eq!(
        //     *partition.bfr[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, 1.0))
        // );
        // assert_eq!(
        //     *partition.bfl[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, 1.0))
        // );
        // assert_eq!(
        //     *partition.bbr[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, -1.0))
        // );
        // assert_eq!(
        //     *partition.bbl[0].transform(&storage),
        //     Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, -1.0))
        // );
    }

    #[test]
    fn test_intersect1() {
        // let mut world = World::new();
        // world.register::<TransformComponent>();
        // world.register::<PrimitiveGeometryComponent>();
        // world.register::<AABBComponent>();

        // let mut entities = vec![];
        // for x in -2..=2 {
        //     for y in -2..=2 {
        //         for z in -2..=2 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //     }
        // }

        // let bvh = Node::_new_from_cubes(&entities, &world.read_storage(), &world.read_storage(), 2)
        //     .unwrap();
        // let entity = bvh.intersected_cube(
        //     &Ray::new(Point3f::new(1.0, 0.0, 10.0), -Vector3f::z_axis().unwrap()),
        //     &world.read_storage(),
        // );
        // assert!(entity
        //     .unwrap()
        //     .position(&world.read_storage())
        //     .almost_eq(&Point3f::new(1.0, 0.0, 2.0)));
    }

    #[test]
    fn test_intersect2() {
        // let mut world = World::new();
        // world.register::<TransformComponent>();
        // world.register::<PrimitiveGeometryComponent>();
        // world.register::<AABBComponent>();

        // let mut entities = vec![];
        // for x in -4..=-2 {
        //     for y in -4..=-2 {
        //         for z in -4..=-2 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //         for z in 2..=4 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //     }
        //     for y in 2..=4 {
        //         for z in -4..=-2 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //         for z in 2..=4 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //     }
        // }
        // for x in 2..=4 {
        //     for y in -4..=-2 {
        //         for z in -4..=-2 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //         for z in 2..=4 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //     }
        //     for y in 2..=4 {
        //         for z in -4..=-2 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //         for z in 2..=4 {
        //             entities.push(Entity::new_unitcube_w(
        //                 &world,
        //                 Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
        //                     .to_superset(),
        //             ));
        //         }
        //     }
        // }

        // let bvh = Node::_new_from_cubes(&entities, &world.read_storage(), &world.read_storage(), 8)
        //     .unwrap();
        // let entity = bvh.intersected_cube(
        //     &Ray::new(Point3f::new(1.0, 0.0, 10.0), -Vector3f::z_axis().unwrap()),
        //     &world.read_storage(),
        // );
        // assert_eq!(entity, None);

        // let entity = bvh.intersected_cube(
        //     &Ray::new(Point3f::origin(), Vector3f::new(1.0, 1.0, 1.0)),
        //     &world.read_storage(),
        // );
        // assert!(entity
        //     .unwrap()
        //     .position(&world.read_storage())
        //     .almost_eq(&Point3f::new(2.0, 2.0, 2.0)));

        // let entity = bvh.intersected_cube(
        //     &Ray::new(
        //         Point3f::new(10.0, 10.0, 10.0),
        //         Vector3f::new(-1.0, -1.0, -1.0),
        //     ),
        //     &world.read_storage(),
        // );
        // assert!(entity
        //     .unwrap()
        //     .position(&world.read_storage())
        //     .almost_eq(&Point3f::new(4.0, 4.0, 4.0)));
    }
}
