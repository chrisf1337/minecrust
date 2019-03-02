use crate::{
    ecs::{entity::Entity, AABBComponent},
    geometry::Ray,
    types::prelude::*,
};
use specs::ReadStorage;
use std::ops::{Index, IndexMut};

struct Chunk {
    side_len: usize,
    entities: Vec<Option<Entity>>,
}

impl Index<(usize, usize, usize)> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, (x, y, z): (usize, usize, usize)) -> &Self::Output {
        &self.entities[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl IndexMut<(usize, usize, usize)> for Chunk {
    fn index_mut(&mut self, (x, y, z): (usize, usize, usize)) -> &mut Self::Output {
        &mut self.entities[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl Chunk {
    fn new(side_len: usize) -> Chunk {
        Chunk {
            side_len,
            entities: vec![None; side_len * side_len * side_len],
        }
    }

    fn intersect(
        &self,
        ray: &Ray,
        storage: &ReadStorage<AABBComponent>,
    ) -> Option<(usize, (f32, Point3f))> {
        ray.intersect_entities_optional(&self.entities, storage)
    }

    fn intersected_entity(
        &self,
        ray: &Ray,
        storage: &ReadStorage<AABBComponent>,
    ) -> Option<Entity> {
        self.intersect(ray, storage)
            .map(|(i, _)| self.entities[i].unwrap())
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
        world.register::<AABBComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(3);
        chunk[(0, 0, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(2, 0, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(0, 2, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(2, 2, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(0, 0, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(2, 0, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(0, 2, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(2, 2, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 2.0)).to_superset(),
            &world,
        ));

        let ray = Ray::new(Point3f::new(1.0, 1.0, 10.0), -Vector3f::z());
        assert_eq!(chunk.intersect(&ray, &world.read_storage()), None);
    }

    #[test]
    fn test_intersect_entities2() {
        let mut world = World::new();
        world.register::<AABBComponent>();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let mut chunk = Chunk::new(3);
        chunk[(0, 0, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(2, 0, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(0, 2, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(2, 2, 0)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 0.0)).to_superset(),
            &world,
        ));
        chunk[(0, 0, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(2, 0, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 0.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(0, 2, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(0.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        chunk[(2, 2, 2)] = Some(Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(2.0, 2.0, 2.0)).to_superset(),
            &world,
        ));
        let entity = Entity::new_unitcube_w(
            Translation3::from(Vector3f::new(1.0, 1.0, 1.0)).to_superset(),
            &world,
        );
        chunk[(1, 1, 1)] = Some(entity);

        let ray = Ray::new(Point3f::new(1.0, 1.0, 10.0), -Vector3f::z());
        assert_eq!(
            chunk.intersected_entity(&ray, &world.read_storage()),
            Some(entity)
        );
    }
}
