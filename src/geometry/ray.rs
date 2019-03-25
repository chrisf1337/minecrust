use crate::{
    ecs::{entity::Entity, AabbComponent},
    geometry::aabb::Aabb,
    types::prelude::*,
};
use specs::ReadStorage;
use std::{f32, mem};

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub origin: Point3f,
    pub direction: Unit<Vector3f>,
}

impl Ray {
    pub fn new(origin: Point3f, direction: Vector3f) -> Ray {
        Ray {
            origin,
            direction: Unit::new_normalize(direction),
        }
    }

    pub fn at(&self, t: f32) -> Point3f {
        self.origin + t * self.direction.as_ref()
    }

    pub fn intersect_aabb(&self, aabb: &Aabb) -> Option<(f32, Point3f)> {
        let min = aabb.min();
        let max = aabb.max();

        let mut tmin = (min.x - self.origin.x) / self.direction.x;
        let mut tmax = (max.x - self.origin.x) / self.direction.x;
        if tmin > tmax {
            mem::swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min.y - self.origin.y) / self.direction.y;
        let mut tymax = (max.y - self.origin.y) / self.direction.y;
        if tymin > tymax {
            mem::swap(&mut tymin, &mut tymax);
        }

        if tmin > tymax || tymin > tmax {
            return None;
        }
        tmin = f32::max(tmin, tymin);
        tmax = f32::min(tmax, tymax);

        let mut tzmin = (min.z - self.origin.z) / self.direction.z;
        let mut tzmax = (max.z - self.origin.z) / self.direction.z;
        if tzmin > tzmax {
            mem::swap(&mut tzmin, &mut tzmax);
        }

        if tmin > tzmax || tzmin > tmax {
            return None;
        }
        tmin = f32::max(tmin, tzmin);
        tmax = f32::min(tmax, tzmax);

        if tmax < 0.0 {
            None
        } else if tmin < 0.0 {
            Some((tmax, self.at(tmax)))
        } else {
            Some((tmin, self.at(tmin)))
        }
    }

    fn intersect_aabb_optional(&self, aabb: &Option<Aabb>) -> Option<(f32, Point3f)> {
        if let Some(aabb) = aabb {
            self.intersect_aabb(aabb)
        } else {
            None
        }
    }

    pub fn intersect_aabbs(&self, aabbs: &[Aabb]) -> Option<(usize, (f32, Point3f))> {
        let aabb = aabbs
            .iter()
            .enumerate()
            .filter_map(|(i, bb)| self.intersect_aabb(bb).map(|r| (i, r)))
            .min_by(|(_, (t1, _)), (_, (t2, _))| t1.partial_cmp(t2).unwrap())?;
        Some(aabb)
    }

    fn intersect_aabbs_optional(&self, aabbs: &[Option<Aabb>]) -> Option<(usize, (f32, Point3f))> {
        let aabb = aabbs
            .iter()
            .enumerate()
            .filter_map(|(i, bb)| self.intersect_aabb_optional(bb).map(|r| (i, r)))
            .min_by(|(_, (t1, _)), (_, (t2, _))| t1.partial_cmp(t2).unwrap())?;
        Some(aabb)
    }

    pub fn intersect_entity(
        &self,
        entity: Entity,
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<(f32, Point3f)> {
        self.intersect_aabb(entity.aabb(storage))
    }

    pub fn intersect_entities(
        &self,
        entities: &[Entity],
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<(usize, (f32, Point3f))> {
        self.intersect_aabbs(
            &entities
                .iter()
                .map(|e| *e.aabb(storage))
                .collect::<Vec<_>>(),
        )
    }

    pub fn intersect_entities_optional(
        &self,
        entities: &[Option<Entity>],
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<(usize, (f32, Point3f))> {
        self.intersect_aabbs_optional(
            &entities
                .iter()
                .map(|e| e.map(|e| *e.aabb(storage)))
                .collect::<Vec<_>>(),
        )
    }

    pub fn closest_entity(
        &self,
        entities: &[Entity],
        storage: &ReadStorage<AabbComponent>,
    ) -> Option<(f32, Entity)> {
        let (i, (t, _)) = self.intersect_entities(entities, storage)?;
        assert!(self.intersect_entity(entities[i], storage).is_some());
        Some((t, entities[i]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{AabbComponent, PrimitiveGeometryComponent, TransformComponent};
    use specs::World;

    #[test]
    fn test_intersect1() {
        let r = Ray::new(Point3f::origin(), Vector3f::x());
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        assert!(intersection
            .unwrap()
            .1
            .almost_eq(&Point3f::new(1.0, 0.0, 0.0)));

        let r = Ray::new(Point3f::origin(), Vector3f::new(1.0, 1.0, 0.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        assert!(intersection
            .unwrap()
            .1
            .almost_eq(&Point3f::new(1.0, 1.0, 0.0)));

        let r = Ray::new(
            Point3f::new(0.0, 10.0, 10.0),
            Vector3f::new(0.0, -1.0, -1.0),
        );
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        assert!(intersection
            .unwrap()
            .1
            .almost_eq(&Point3f::new(0.0, 1.0, 1.0)));
    }

    #[test]
    fn test_intersect2() {
        let r = Ray::new(Point3f::new(-1.0, -0.5, 2.0), Vector3f::new(1.0, 0.0, -1.0));
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        println!("{:?}", intersection);
        assert!(intersection
            .unwrap()
            .1
            .almost_eq(&Point3f::new(0.0, -0.5, 1.0)));
    }

    #[test]
    fn test_intersect3() {
        let ray = Ray::new(
            Point3f::new(10.0, 10.0, 10.0),
            Vector3f::new(-1.0, -1.0, -1.0),
        );
        let intersection = ray.intersect_aabbs(&[Aabb::new_min_max(
            Point3f::new(3.0, 3.0, 3.0),
            Point3f::new(4.0, 4.0, 4.0),
        )]);
        println!("{:#?}", intersection);
    }

    #[test]
    fn test_no_intersect1() {
        let r = Ray::new(Point3f::new(2.0, 0.0, 0.0), Vector3f::x());
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_none());
    }

    #[test]
    fn test_no_intersect2() {
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let dir = Vector3f::new(0.99, 0.0, 1.0);
        let r = Ray::new(Point3f::new(0.0, 0.0, 2.0), dir);
        assert_eq!(r.intersect_aabb(&aabb), None);
    }

    #[test]
    fn test_intersect_aabbs1() {
        let r = Ray::new(Point3f::new(-2.0, 0.5, -0.5), Vector3f::x());
        let intersection = r
            .intersect_aabbs(&[
                Aabb::new_min_max(Point3f::new(0.0, 0.0, -1.0), Point3f::new(1.0, 1.0, 0.0)),
                Aabb::new_min_max(Point3f::new(-1.0, 0.0, -1.0), Point3f::new(0.0, 1.0, 0.0)),
            ])
            .unwrap();
        assert_eq!(intersection.0, 1);
    }

    #[test]
    fn test_intersect_aabbs2() {
        let r = Ray::new(Point3f::new(-2.0, 1.5, -0.5), Vector3f::x());
        assert_eq!(
            r.intersect_aabbs(&[
                Aabb::new_min_max(Point3f::new(0.0, 0.0, -1.0), Point3f::new(1.0, 1.0, 0.0)),
                Aabb::new_min_max(Point3f::new(-1.0, 0.0, -1.0), Point3f::new(0.0, 1.0, 0.0)),
            ]),
            None
        );
    }

    #[test]
    fn test_closest_entity() {
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<AabbComponent>();
        world.register::<PrimitiveGeometryComponent>();

        let r = Ray::new(Point3f::origin(), Vector3f::x());
        let entities = vec![
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(2.0, 0.0, 0.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(1.0, 0.0, 0.0)),
                &world,
            ),
            Entity::new_unitcube_w(
                Transform3f::new_with_translation(Vector3f::new(3.0, 0.0, 0.0)),
                &world,
            ),
        ];
        assert_eq!(
            r.closest_entity(&entities, &world.read_storage())
                .unwrap()
                .1,
            entities[1]
        );
    }
}
