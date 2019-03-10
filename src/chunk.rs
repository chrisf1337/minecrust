use crate::{
    ecs::{entity::Entity, AABBComponent, TransformComponent},
    geometry::Ray,
    types::prelude::*,
};
use specs::ReadStorage;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut, Index, IndexMut},
};

#[derive(Debug, Clone)]
struct Vector2D<T> {
    side_len: usize,
    storage: Vec<T>,
}

impl<T: Clone> Vector2D<T> {
    fn new(side_len: usize, t: T) -> Vector2D<T> {
        assert_eq!(side_len % 2, 1, "side_len must be odd");
        Vector2D {
            side_len,
            storage: vec![t; side_len * side_len],
        }
    }
}

impl<T: Clone + Default> Vector2D<T> {
    fn new_default(side_len: usize) -> Vector2D<T> {
        Vector2D::new(side_len, T::default())
    }
}

impl<T> Index<usize> for Vector2D<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.storage[i]
    }
}

impl<T> IndexMut<usize> for Vector2D<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.storage[i]
    }
}

impl<T> Index<Vector2f> for Vector2D<T> {
    type Output = T;

    fn index(&self, i: Vector2f) -> &Self::Output {
        assert!(i.almost_is_int());
        &self[(i.x as i32, i.y as i32)]
    }
}

impl<T> IndexMut<Vector2f> for Vector2D<T> {
    fn index_mut(&mut self, i: Vector2f) -> &mut Self::Output {
        assert!(i.almost_is_int());
        &mut self[(i.x as i32, i.y as i32)]
    }
}

impl<T> Index<(i32, i32)> for Vector2D<T> {
    type Output = T;

    fn index(&self, (x, y): (i32, i32)) -> &Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        &self.storage[x + y * self.side_len]
    }
}

impl<T> IndexMut<(i32, i32)> for Vector2D<T> {
    fn index_mut(&mut self, (x, y): (i32, i32)) -> &mut Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        &mut self.storage[x + y * self.side_len]
    }
}

impl<T> Deref for Vector2D<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<T> DerefMut for Vector2D<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage
    }
}

#[derive(Debug, Clone)]
struct Vector3D<T> {
    side_len: usize,
    storage: Vec<T>,
}

impl<T: Clone> Vector3D<T> {
    fn new(side_len: usize, t: T) -> Vector3D<T> {
        assert_eq!(side_len % 2, 1, "side_len must be odd");
        Vector3D {
            side_len,
            storage: vec![t; side_len * side_len * side_len],
        }
    }
}

impl<T: Clone + Default> Vector3D<T> {
    fn new_default(side_len: usize) -> Vector3D<T> {
        Vector3D::new(side_len, T::default())
    }
}

impl<T> Index<usize> for Vector3D<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.storage[i]
    }
}

impl<T> IndexMut<usize> for Vector3D<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.storage[i]
    }
}

impl<T> Index<Vector3f> for Vector3D<T> {
    type Output = T;

    fn index(&self, i: Vector3f) -> &Self::Output {
        assert!(i.almost_is_int());
        &self[(i.x as i32, i.y as i32, i.z as i32)]
    }
}

impl<T> IndexMut<Vector3f> for Vector3D<T> {
    fn index_mut(&mut self, i: Vector3f) -> &mut Self::Output {
        assert!(i.almost_is_int());
        &mut self[(i.x as i32, i.y as i32, i.z as i32)]
    }
}

impl<T> Index<(i32, i32, i32)> for Vector3D<T> {
    type Output = T;

    fn index(&self, (x, y, z): (i32, i32, i32)) -> &Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &self.storage[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl<T> IndexMut<(i32, i32, i32)> for Vector3D<T> {
    fn index_mut(&mut self, (x, y, z): (i32, i32, i32)) -> &mut Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &mut self.storage[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl<T> Deref for Vector3D<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<T> DerefMut for Vector3D<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage
    }
}

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

    fn slices(&self, face: Face) -> Vec<Vector2D<Entity>> {
        let mut slices = vec![];
        // let mut seen = vec![];
        let side_len = self.entities.side_len as i32;
        // match face {
        //     Face::Top => {
        //         for x in -side_len as i32 / 2..=side_len / 2 {
        //             for y in -side_len as i32 / 2..=side_len / 2 {}
        //         }
        //     }
        // }
        slices
    }
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
