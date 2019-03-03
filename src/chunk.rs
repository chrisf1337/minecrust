use crate::{
    ecs::{entity::Entity, AABBComponent, TransformComponent},
    geometry::Ray,
    types::prelude::*,
};
use specs::ReadStorage;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone)]
pub struct Chunk {
    center: Point3f,
    side_len: usize,
    entities: Vec<Option<Entity>>,
}

impl Index<Point3f> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, i: Point3f) -> &Self::Output {
        let v = i - self.center;
        assert!(v.almost_is_int());
        &self[(v.x as i32, v.y as i32, v.z as i32)]
    }
}

impl IndexMut<Point3f> for Chunk {
    fn index_mut(&mut self, i: Point3f) -> &mut Self::Output {
        let v = i - self.center;
        assert!(v.almost_is_int());
        &mut self[(v.x as i32, v.y as i32, v.z as i32)]
    }
}

impl Index<(i32, i32, i32)> for Chunk {
    type Output = Option<Entity>;

    fn index(&self, (x, y, z): (i32, i32, i32)) -> &Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &self.entities[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl IndexMut<(i32, i32, i32)> for Chunk {
    fn index_mut(&mut self, (x, y, z): (i32, i32, i32)) -> &mut Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &mut self.entities[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl Chunk {
    pub fn new(center: Point3f, side_len: usize) -> Chunk {
        assert_eq!(side_len % 2, 1, "side_len must be odd");
        Chunk {
            center,
            side_len,
            entities: vec![None; side_len * side_len * side_len],
        }
    }

    pub fn insert(&mut self, entity: Entity, storage: &ReadStorage<TransformComponent>) {
        self[entity.position(storage)] = Some(entity);
    }

    fn intersect(
        &self,
        ray: &Ray,
        storage: &ReadStorage<AABBComponent>,
    ) -> Option<(usize, (f32, Point3f))> {
        ray.intersect_entities_optional(&self.entities, storage)
    }

    pub fn intersected_entity(
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
        world.register::<AABBComponent>();
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
        world.register::<AABBComponent>();
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
        world.register::<AABBComponent>();
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
}
