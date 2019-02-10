use crate::{
    ecs::{entity::Entity, AABBComponent},
    geometry::aabb::AABB,
    types::prelude::*,
};
use specs::ReadStorage;
use std::{f32, mem};

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub origin: Point3f,
    pub direction: Vector3f,
}

impl Ray {
    pub fn new(origin: Point3f, direction: Vector3f) -> Ray {
        Ray { origin, direction }
    }

    pub fn at(&self, t: f32) -> Point3f {
        self.origin + t * self.direction
    }

    pub fn intersect_aabb_t(&self, aabb: &AABB) -> Option<(f32, Point3f)> {
        let min = aabb.min;
        let max = aabb.max;

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

    pub fn intersect_aabb(&self, aabb: &AABB) -> Option<Point3f> {
        self.intersect_aabb_t(aabb).map(|(_, p)| p)
    }

    pub fn intersect_aabbs(&self, aabbs: &[AABB]) -> Option<(usize, Point3f)> {
        let mut aabbs: Vec<_> = aabbs
            .iter()
            .filter_map(|bb| self.intersect_aabb_t(bb))
            .enumerate()
            .collect();
        aabbs.sort_unstable_by(|(_, (t1, _)), (_, (t2, _))| t1.partial_cmp(t2).unwrap());
        aabbs.get(0).map(|(i, (_, p))| (*i, *p))
    }

    pub fn intersect_entity_t(
        &self,
        entity: Entity,
        storage: &ReadStorage<AABBComponent>,
    ) -> Option<(f32, Point3f)> {
        self.intersect_aabb_t(entity.aabb(storage))
    }

    pub fn intersect_entities(
        &self,
        entities: &[Entity],
        storage: &ReadStorage<AABBComponent>,
    ) -> Option<(usize, Point3f)> {
        self.intersect_aabbs(
            &entities
                .iter()
                .map(|e| *e.aabb(storage))
                .collect::<Vec<_>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::prelude::*;

    #[test]
    fn test_intersect1() {
        let r = Ray::new(Point3f::origin(), Vector3f::x());
        let aabb = AABB::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        assert!(intersection
            .unwrap()
            .almost_eq(&Point3f::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_intersect2() {
        let r = Ray::new(Point3f::new(-1.0, -0.5, 2.0), Vector3f::new(1.0, 0.0, -1.0));
        let aabb = AABB::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_some());
        println!("{:?}", intersection);
        assert!(intersection
            .unwrap()
            .almost_eq(&Point3f::new(0.0, -0.5, 1.0)));
    }

    #[test]
    fn test_no_intersect1() {
        let r = Ray::new(Point3f::new(2.0, 0.0, 0.0), Vector3f::x());
        let aabb = AABB::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let intersection = r.intersect_aabb(&aabb);
        assert!(intersection.is_none());
    }

    #[test]
    fn test_no_intersect2() {
        let aabb = AABB::new(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));
        let dir = Vector3f::new(0.99, 0.0, 1.0);
        let r = Ray::new(Point3f::new(0.0, 0.0, 2.0), dir);
        assert_eq!(r.intersect_aabb(&aabb), None);
    }

    #[test]
    fn test_intersect_aabbs1() {
        let r = Ray::new(Point3f::new(-2.0, 0.5, -0.5), Vector3f::x());
        let intersection = r
            .intersect_aabbs(&[
                AABB::new(Point3f::new(0.0, 0.0, -1.0), Point3f::new(1.0, 1.0, 0.0)),
                AABB::new(Point3f::new(-1.0, 0.0, -1.0), Point3f::new(0.0, 1.0, 0.0)),
            ])
            .unwrap();
        assert_eq!(intersection.0, 1);
    }

    #[test]
    fn test_intersect_aabbs2() {
        let r = Ray::new(Point3f::new(-2.0, 1.5, -0.5), Vector3f::x());
        assert_eq!(
            r.intersect_aabbs(&[
                AABB::new(Point3f::new(0.0, 0.0, -1.0), Point3f::new(1.0, 1.0, 0.0)),
                AABB::new(Point3f::new(-1.0, 0.0, -1.0), Point3f::new(0.0, 1.0, 0.0)),
            ]),
            None
        );
    }
}
