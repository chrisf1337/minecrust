use crate::{
    ecs::{entity::Entity, AabbComponent, TransformComponent},
    geometry::Ray,
    types::prelude::*,
    vector::{Vector2D, Vector3D},
    vulkan::Vertex3f,
};
use specs::ReadStorage;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub struct Chunk {
    center: Point3f,
    entities: Vector3D<Option<Entity>>,
    slices: HashMap<Face, Vector2D<Option<Entity>>>,
}

impl Chunk {
    pub fn new(center: Point3f, side_len: usize) -> Chunk {
        Chunk {
            center,
            entities: Vector3D::new_default(side_len),
            slices: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, storage: &ReadStorage<TransformComponent>) {
        self[entity.position(storage)] = Some(entity);
    }

    fn intersect(
        &self,
        ray: &Ray,
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<(usize, (f32, Point3f))> {
        ray.intersect_entities_optional(&self.entities, storage)
    }

    pub fn intersected_entity(
        &self,
        ray: &Ray,
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<Entity> {
        self.intersect(ray, storage)
            .map(|(i, _)| self.entities[i].unwrap())
    }

    /// WIP: For face merging
    fn slices(&self, face: Face) -> Vec<Vector2D<Option<Entity>>> {
        let mut slices = vec![];
        let mut seen: Vector2D<Option<(usize, Entity)>> =
            Vector2D::new_default(self.entities.side_len);
        let side_len = self.entities.side_len as i32;
        match face {
            Face::Top => {
                for x in -side_len as i32 / 2..=side_len / 2 {
                    for z in -side_len as i32 / 2..=side_len / 2 {
                        for (i, y) in (-side_len as i32 / 2..=side_len / 2).rev().enumerate() {
                            if let Some(ety) = self.entities[(x, y, z)] {
                                if seen[(x, z)].is_none() {
                                    seen[(x, z)] = Some((i, ety))
                                }
                            }
                        }
                    }
                }
            }
            _ => unimplemented!(),
        }
        slices
    }

    // fn vtx_data(&self, transform_storage: &ReadStorage<TransformComponent>) -> Vec<Vertex3f> {
    //     let mut vertices = vec![];
    //     let side_len = self.entities.side_len as i32;
    //     for x in -side_len / 2..=side_len / 2 {
    //         for y in -side_len / 2..=side_len / 2 {
    //             for z in -side_len / 2..=side_len / 2 {
    //                 if let Some(ety) = self[(x, y, z)] {
    //                     match
    //                     let transform = Translation3f::from(self.center + Vector3f::new(x as f32, y as f32, z as f32)).to_superset();
    //                     vertices.extend_from_slice()
    //                 }
    //             }
    //         }
    //     }
    // }
}

impl Index<(i32, i32, i32)> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, i: (i32, i32, i32)) -> &Self::Output {
        &self.entities[i]
    }
}

impl IndexMut<(i32, i32, i32)> for Chunk {
    fn index_mut(&mut self, i: (i32, i32, i32)) -> &mut Self::Output {
        &mut self.entities[i]
    }
}

impl Index<Vector3f> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, i: Vector3f) -> &Self::Output {
        &self.entities[i]
    }
}

impl IndexMut<Vector3f> for Chunk {
    fn index_mut(&mut self, i: Vector3f) -> &mut Self::Output {
        &mut self.entities[i]
    }
}

impl Index<Point3f> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, i: Point3f) -> &Self::Output {
        &self.entities[i - self.center]
    }
}

impl IndexMut<Point3f> for Chunk {
    fn index_mut(&mut self, i: Point3f) -> &mut Self::Output {
        &mut self.entities[i - self.center]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{PrimitiveGeometryComponent, TransformComponent};
    use alga::general::SubsetOf;
    use specs::World;

    #[test]
    fn test_intersect_entities1() {
        let mut world = World::new();
        world.register::<AabbComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(Point3f::new(1.0, 1.0, 1.0), 3);
        chunk[(-1, -1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(1, -1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(-1, 1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(1, 1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(-1, -1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(1, -1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(-1, 1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(1, 1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 2.0)).to_superset(),
            &world,
        ));

        let ray = Ray::new(Point3f::new(1.0, 1.0, 10.0), -Vector3f::z());
        assert_eq!(chunk.intersect(&ray, &world.read_storage()), None);
    }

    #[test]
    fn test_intersect_entities2() {
        let mut world = World::new();
        world.register::<AabbComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(Point3f::new(1.0, 1.0, 1.0), 3);
        chunk[(-1, -1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(1, -1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(-1, 1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(1, 1, -1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(-1, -1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(1, -1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(-1, 1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(1, 1, 1)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        let entity = Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(1.0, 1.0, 1.0)).to_superset(),
            &world,
        );
        chunk[(0, 0, 0)] = Some(entity);

        let ray = Ray::new(Point3f::new(1.0, 1.0, 10.0), -Vector3f::z());
        assert_eq!(
            chunk.intersected_entity(&ray, &world.read_storage()),
            Some(entity)
        );
    }

    #[test]
    fn test_insert() {
        let mut world = World::new();
        world.register::<AabbComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(Point3f::new(1.0, -1.0, 1.0), 9);
        let entity = Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(1.0, 1.0, 1.0)).to_superset(),
            &world,
        );
        chunk.insert(entity, &world.read_storage());
        assert_eq!(chunk[(0, 2, 0)], Some(entity));
        assert_eq!(chunk[Point3f::new(1.0, 1.0, 1.0)], Some(entity));

        let entity = Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(3.0, 0.0, -1.0)).to_superset(),
            &world,
        );
        chunk.insert(entity, &world.read_storage());
        assert_eq!(chunk[(2, 1, -2)], Some(entity));
        assert_eq!(chunk[Point3f::new(3.0, 0.0, -1.0)], Some(entity));
    }

    #[test]
    #[should_panic]
    fn test_insert_panic() {
        let mut world = World::new();
        world.register::<AabbComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(Point3f::origin(), 3);
        chunk.insert(
            Entity::new_unitcube_w(
                Translation3::from(Vector3f::new(2.0, 0.0, 0.0)).to_superset(),
                &world,
            ),
            &world.read_storage(),
        );
    }

    #[test]
    #[should_panic]
    fn test_index_panic() {
        let chunk = Chunk::new(Point3f::origin(), 3);
        let _ = chunk[Point3f::new(0.0, -2.0, 0.0)];
    }

    #[test]
    fn test_slice() {
        let mut world = World::new();
        world.register::<AabbComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(Point3f::origin(), 3);
        let ety1 = Entity::new_unitcube_w(
            Translation3f::from(Vector3f::new(-1.0, 1.0, -1.0)).to_superset(),
            &world,
        );
        chunk.insert(ety1, &world.read_storage());
        chunk.insert(
            Entity::new_unitcube_w(
                Translation3f::from(Vector3f::new(-1.0, 0.0, -1.0)).to_superset(),
                &world,
            ),
            &world.read_storage(),
        );
        let ety2 = Entity::new_unitcube_w(
            Translation3f::from(Vector3f::new(0.0, 0.0, -1.0)).to_superset(),
            &world,
        );
        chunk.insert(ety2, &world.read_storage());

        let mut vec = Vector2D::new_default(3);
        vec[(-1, -1)] = Some(ety1);
        vec[(0, -1)] = Some(ety2);
        // assert_eq!(chunk.slices(Face::Top), vec);
    }
}
