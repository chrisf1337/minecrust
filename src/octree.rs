use crate::{
    ecs::{entity::Entity, AABBComponent, PrimitiveGeometryComponent, TransformComponent},
    geometry::{Axis, Ray, AABB, AAP},
    types::prelude::*,
};
use num_traits::identities::Zero;
use specs::ReadStorage;

const CHILD_NODE_MAX_SIZE: usize = 8;

#[derive(Debug, Clone)]
struct Node {
    center: Point3f,
    aabb: AABB,
    children: Option<NodeOctPartition>,
    entities: Vec<Entity>,
}

impl Node {
    fn _new_from_cubes(
        entities: &[Entity],
        center: Point3f,
        aabb: AABB,
        transform_storage: &ReadStorage<TransformComponent>,
        aabb_storage: &ReadStorage<AABBComponent>,
        child_node_max_size: usize,
    ) -> Option<Node> {
        if entities.is_empty() {
            return None;
        }
        if entities.len() <= child_node_max_size {
            return Some(Node {
                center,
                aabb,
                children: None,
                entities: entities.to_vec(),
            });
        }
        let (node_entities, partition) =
            partition_entities(entities, &center, transform_storage, aabb_storage);
        // let tfr = Node::_new_from_cubes(&partition.tfr, , aabb: AABB, transform_storage: &ReadStorage<TransformComponent>, aabb_storage: &ReadStorage<AABBComponent>, child_node_max_size: usize);
        None
    }
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

#[derive(Default, Debug, Clone)]
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

fn partition_entities(
    entities: &[Entity],
    point: &Point3f,
    transform_storage: &ReadStorage<TransformComponent>,
    aabb_storage: &ReadStorage<AABBComponent>,
) -> (Vec<Entity>, OctPartition) {
    let x_plane = AAP::new(Axis::X, point.x);
    let y_plane = AAP::new(Axis::Y, point.y);
    let z_plane = AAP::new(Axis::Z, point.z);
    let mut node_entities = vec![];
    let mut oct_partition = OctPartition::default();
    for &entity in entities {
        let aabb = entity.aabb(aabb_storage);
        if x_plane.intersects_aabb(&aabb)
            || y_plane.intersects_aabb(&aabb)
            || z_plane.intersects_aabb(&aabb)
        {
            node_entities.push(entity);
        } else {
            let v = entity.position(transform_storage) - point;
            if v.x >= 0.0 {
                if v.y >= 0.0 {
                    if v.z >= 0.0 {
                        oct_partition.tfr.push(entity);
                    } else {
                        oct_partition.tbr.push(entity);
                    }
                } else if v.z >= 0.0 {
                    oct_partition.bfr.push(entity);
                } else {
                    oct_partition.bbr.push(entity);
                }
            } else if v.y >= 0.0 {
                if v.z >= 0.0 {
                    oct_partition.tfl.push(entity);
                } else {
                    oct_partition.tbl.push(entity);
                }
            } else if v.z >= 0.0 {
                oct_partition.bfl.push(entity);
            } else {
                oct_partition.bbl.push(entity);
            }
        }
    }
    (node_entities, oct_partition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::prelude::*;
    use alga::general::SubsetOf;
    use specs::World;

    #[test]
    fn test_partition_entities1() {
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<AABBComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let entities = vec![
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, 1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, -1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, 1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, -1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, 1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, -1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, 1.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, -1.0)),
                &world,
            ),
        ];

        let (node_entities, partition) = partition_entities(
            &entities,
            &Point3f::origin(),
            &world.read_storage(),
            &world.read_storage(),
        );
        let storage = world.read_storage();
        assert_eq!(
            *partition.tfr[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, 1.0))
        );
        assert_eq!(
            *partition.tfl[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, 1.0))
        );
        assert_eq!(
            *partition.tbr[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(1.0, 1.0, -1.0))
        );
        assert_eq!(
            *partition.tbl[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(-1.0, 1.0, -1.0))
        );
        assert_eq!(
            *partition.bfr[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, 1.0))
        );
        assert_eq!(
            *partition.bfl[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, 1.0))
        );
        assert_eq!(
            *partition.bbr[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(1.0, -1.0, -1.0))
        );
        assert_eq!(
            *partition.bbl[0].transform(&storage),
            Transform3f::new_with_translation(Vector3f::new(-1.0, -1.0, -1.0))
        );

        assert!(node_entities.is_empty());
    }

    #[test]
    fn test_partition_entities2() {
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<AABBComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut entities = vec![];
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    entities.push(Entity::new_unitcube_w(
                        Transform3f::new_with_translation(Vector3f::new(
                            x as f32, y as f32, z as f32,
                        )),
                        &world,
                    ));
                }
            }
        }

        let (node_entities, partition) = partition_entities(
            &entities,
            &Point3f::new(0.0, 0.0, 0.0),
            &world.read_storage(),
            &world.read_storage(),
        );
        for entity in &node_entities {
            dbg!(entity.position(&world.read_storage()));
        }
        dbg!(&partition);
        for i in -1..=1 {
            assert!(node_entities.iter().any(|entity| entity
                .position(&world.read_storage())
                .almost_eq(&Point3f::new(i as f32, 0.0, 0.0))));
            assert!(node_entities.iter().any(|entity| entity
                .position(&world.read_storage())
                .almost_eq(&Point3f::new(0.0, i as f32, 0.0))));
            assert!(node_entities.iter().any(|entity| entity
                .position(&world.read_storage())
                .almost_eq(&Point3f::new(0.0, 0.0, i as f32))));
        }
        assert_eq!(node_entities.len(), 19);

        assert_eq!(partition.tfl.len(), 1);
        assert!(partition.tfl[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(-1.0, 1.0, 1.0)));

        assert_eq!(partition.tfr.len(), 1);
        assert!(partition.tfr[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, 1.0, 1.0)));

        assert_eq!(partition.tbl.len(), 1);
        assert!(partition.tbl[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(-1.0, 1.0, -1.0)));

        assert_eq!(partition.tbr.len(), 1);
        assert!(partition.tbr[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, 1.0, -1.0)));

        assert_eq!(partition.bfl.len(), 1);
        assert!(partition.bfl[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(-1.0, -1.0, 1.0)));

        assert_eq!(partition.bfr.len(), 1);
        assert!(partition.bfr[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, -1.0, 1.0)));

        assert_eq!(partition.bbl.len(), 1);
        assert!(partition.bbl[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(-1.0, -1.0, -1.0)));

        assert_eq!(partition.bbr.len(), 1);
        assert!(partition.bbr[0]
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, -1.0, -1.0)));
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
