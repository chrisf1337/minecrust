//! Probably not going to use this for selection for now.

use crate::{
    ecs::{entity::Entity, AabbComponent},
    geometry::{Aabb, Axis, Ray, AAP},
    types::{prelude::*, Octants, OctantIndex},
    utils::f32,
};
use specs::ReadStorage;

const TERMINAL_NODE_MAX_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct Node {
    aabb: Aabb,
    children: Option<Octants<Box<Node>>>,
    entities: Vec<Entity>,
}

impl Node {
    pub fn empty(aabb: Aabb) -> Node {
        Node {
            aabb,
            children: None,
            entities: vec![],
        }
    }

    pub fn new_from_entities(
        entities: &[Entity],
        aabb: Aabb,
        aabb_storage: &ReadStorage<AabbComponent>,
    ) -> Node {
        Node::_new_from_entities(entities, aabb, aabb_storage, TERMINAL_NODE_MAX_SIZE)
    }

    fn _new_from_entities(
        entities: &[Entity],
        aabb: Aabb,
        aabb_storage: &ReadStorage<AabbComponent>,
        child_node_max_size: usize,
    ) -> Node {
        if entities.is_empty() {
            return Node {
                aabb,
                children: None,
                entities: vec![],
            };
        }
        if entities.len() <= child_node_max_size {
            return Node {
                aabb,
                children: None,
                entities: entities.to_vec(),
            };
        }
        let (node_entities, children) =
            partition_children(entities, &aabb, aabb_storage, child_node_max_size);
        Node {
            aabb,
            children: Some(children),
            entities: node_entities,
        }
    }

    fn is_terminal(&self) -> bool {
        self.children.is_none()
    }

    pub fn intersect_entity(
        &self,
        ray: &Ray,
        aabb_storage: &ReadStorage<AabbComponent>,
    ) -> Option<Entity> {
        self._intersect_entity(ray, aabb_storage).map(|x| x.1)
    }

    fn _intersect_entity(
        &self,
        ray: &Ray,
        aabb_storage: &ReadStorage<AabbComponent>,
    ) -> Option<(f32, Entity)> {
        if self.is_terminal() {
            return ray.closest_entity(&self.entities, aabb_storage);
        }

        let mut a = 0;
        let mut ox = ray.origin.x;
        let mut oy = ray.origin.y;
        let mut oz = ray.origin.z;
        let mut dx = ray.direction.x;
        let mut dy = ray.direction.y;
        let mut dz = ray.direction.z;
        let center_x = self.aabb.center.x;
        let center_y = self.aabb.center.y;
        let center_z = self.aabb.center.z;

        if ray.direction.x < 0.0 {
            ox = center_x - ray.origin.x;
            dx = -ray.direction.x;
            a |= 4;
        }

        if ray.direction.y < 0.0 {
            oy = center_y - ray.origin.y;
            dy = -ray.direction.y;
            a |= 2;
        }

        if ray.direction.z < 0.0 {
            oz = center_z - ray.origin.z;
            dz = -ray.direction.z;
            a |= 1;
        }

        let min = self.aabb.min();
        let max = self.aabb.max();

        let tx0 = if dx != 0.0 {
            (min.x - ox) / dx
        } else if min.x - ox <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let ty0 = if dy != 0.0 {
            (min.y - oy) / dy
        } else if min.y - oy <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let tz0 = if dz != 0.0 {
            (min.z - oz) / dz
        } else if min.z - oz <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };

        let tx1 = if dx != 0.0 {
            (max.x - ox) / dx
        } else if max.x - ox < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let ty1 = if dy != 0.0 {
            (max.y - oy) / dy
        } else if max.y - oy < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let tz1 = if dz != 0.0 {
            (max.z - oz) / dz
        } else if max.z - oz < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };

        if f32::max_many(&[tx0, ty0, tz0]) < f32::min_many(&[tx1, ty1, tz1]) {
            return self.proc_subtree(ray, aabb_storage, (tx0, ty0, tz0), (tx1, ty1, tz1), a);
        }
        None
    }

    fn proc_subtree(
        &self,
        ray: &Ray,
        aabb_storage: &ReadStorage<AabbComponent>,
        (tx0, ty0, tz0): (f32, f32, f32),
        (tx1, ty1, tz1): (f32, f32, f32),
        a: usize,
    ) -> Option<(f32, Entity)> {
        // println!("proc_subtree({:?}, {:?})", (tx0, ty0, tz0), (tx1, ty1, tz1));
        // for entity in &self.entities {
        //     println!("{:?}", entity.aabb(aabb_storage).center());
        // }
        if tx1 < 0.0 || ty1 < 0.0 || tz1 < 0.0 {
            return None;
        }
        let entity_candidate = ray.closest_entity(&self.entities, aabb_storage);
        // println!(
        //     "candidate {:?}",
        //     entity_candidate.map(|(_, e)| e.aabb(aabb_storage).center())
        // );
        if self.is_terminal() {
            return entity_candidate;
        }
        let mut txm = 0.5 * (tx0 + tx1);
        let mut tym = 0.5 * (ty0 + ty1);
        let mut tzm = 0.5 * (tz0 + tz1);

        let min = self.aabb.min();
        let max = self.aabb.max();

        if txm.is_nan() {
            txm = if ray.origin.x < (min.x + max.x) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }
        if tym.is_nan() {
            tym = if ray.origin.y < (min.y + max.y) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }
        if tzm.is_nan() {
            tzm = if ray.origin.z < (min.z + max.z) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }

        let mut entity: Option<(f32, Entity)> = None;
        let mut curr_node = first_node((tx0, ty0, tz0), (txm, tym, tzm));
        // println!("curr_node({} {})", curr_node, curr_node ^ a,);
        loop {
            match curr_node {
                0 => {
                    if let Some(ref children) = self.children {
                        let node = &children[a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (tx0, ty0, tz0),
                                (txm, tym, tzm),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(txm, 4), (tym, 2), (tzm, 1)]);
                    }
                }
                1 => {
                    if let Some(ref children) = self.children {
                        let node = &children[1 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (tx0, ty0, tzm),
                                (txm, tym, tz1),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(txm, 5), (tym, 3), (tz1, 8)]);
                    }
                }
                2 => {
                    if let Some(ref children) = self.children {
                        let node = &children[2 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (tx0, tym, tz0),
                                (txm, ty1, tzm),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(txm, 6), (ty1, 8), (tzm, 3)]);
                    }
                }
                3 => {
                    if let Some(ref children) = self.children {
                        let node = &children[3 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (tx0, tym, tzm),
                                (txm, ty1, tz1),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(txm, 7), (ty1, 8), (tz1, 8)]);
                    }
                }
                4 => {
                    if let Some(ref children) = self.children {
                        let node = &children[4 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (txm, ty0, tz0),
                                (tx1, tym, tzm),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(tx1, 8), (tym, 6), (tzm, 5)]);
                    }
                }
                5 => {
                    if let Some(ref children) = self.children {
                        let node = &children[5 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (txm, ty0, tzm),
                                (tx1, tym, tz1),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(tx1, 8), (tym, 7), (tz1, 8)]);
                    }
                }
                6 => {
                    if let Some(ref children) = self.children {
                        let node = &children[6 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (txm, tym, tz0),
                                (tx1, ty1, tzm),
                                a,
                            ),
                        );
                        curr_node = new_node(&[(tx1, 8), (ty1, 8), (tzm, 7)]);
                    }
                }
                7 => {
                    if let Some(ref children) = self.children {
                        let node = &children[7 ^ a];
                        entity = choose_entity(
                            entity,
                            node.proc_subtree(
                                ray,
                                aabb_storage,
                                (txm, tym, tzm),
                                (tx1, ty1, tz1),
                                a,
                            ),
                        );
                        curr_node = 8;
                    }
                }
                8 => break,
                _ => unreachable!(),
            }
            if entity.is_some() {
                return choose_entity(entity, entity_candidate);
            }
        }
        choose_entity(entity, entity_candidate)
    }

    pub fn insert(&mut self, entity: Entity, aabb_storage: &ReadStorage<AabbComponent>) {
        if self.is_terminal() {
            if self.entities.len() >= TERMINAL_NODE_MAX_SIZE {
                let (node_entities, children) = partition_children(
                    &self.entities,
                    &self.aabb,
                    aabb_storage,
                    TERMINAL_NODE_MAX_SIZE,
                );
                self.entities = node_entities;
                self.children = Some(children);
            } else {
                self.entities.push(entity);
            }
        } else {
            let oct_idx = octant_index(&(self.aabb.center - entity.aabb(&aabb_storage).center));
            let mut parent = self;
            let mut child = &mut parent.children.as_mut().unwrap()[oct_idx];
            while !child.is_terminal() {
                parent = child;
                let oct_idx =
                    octant_index(&(parent.aabb.center - entity.aabb(&aabb_storage).center));
                child = &mut parent.children.as_mut().unwrap()[oct_idx];
            }
        }
    }
}

fn first_node((tx0, ty0, tz0): (f32, f32, f32), (txm, tym, tzm): (f32, f32, f32)) -> usize {
    let mut n = 0;
    match f32::max_index(&[tx0, ty0, tz0]) {
        2 => {
            n |= ((txm < tz0) as usize) << 2;
            n |= ((tym < tz0) as usize) << 1;
        }
        1 => {
            n |= ((txm < ty0) as usize) << 2;
            n |= (tzm < ty0) as usize;
        }
        0 => {
            n |= ((tym < tx0) as usize) << 1;
            n |= (tzm < tx0) as usize;
        }
        _ => unreachable!(),
    }
    assert!(n <= 7);
    n
}

fn new_node(pairs: &[(f32, usize)]) -> usize {
    assert!(!pairs.is_empty());
    pairs
        .iter()
        .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap())
        .unwrap()
        .1
}

fn choose_entity(a: Option<(f32, Entity)>, b: Option<(f32, Entity)>) -> Option<(f32, Entity)> {
    match (a, b) {
        (_, None) => a,
        (None, _) => b,
        (Some((ta, _)), Some((tb, _))) => {
            if ta < tb {
                a
            } else {
                b
            }
        }
    }
}

fn octant_index(v: &Vector3f) -> OctantIndex {
    if v.x >= 0.0 {
        if v.y >= 0.0 {
            if v.z >= 0.0 {
                OctantIndex::Tfr
            } else {
                OctantIndex::Tbr
            }
        } else if v.z >= 0.0 {
            OctantIndex::Bfr
        } else {
            OctantIndex::Bbr
        }
    } else if v.y >= 0.0 {
        if v.z >= 0.0 {
            OctantIndex::Tfl
        } else {
            OctantIndex::Tbl
        }
    } else if v.z >= 0.0 {
        OctantIndex::Bfl
    } else {
        OctantIndex::Bbl
    }
}

fn partition_entities(
    entities: &[Entity],
    point: &Point3f,
    aabb_storage: &ReadStorage<AabbComponent>,
) -> (Vec<Entity>, Octants<Vec<Entity>>) {
    let x_plane = AAP::new(Axis::X, point.x);
    let y_plane = AAP::new(Axis::Y, point.y);
    let z_plane = AAP::new(Axis::Z, point.z);
    let mut node_entities = vec![];
    let mut oct_partition: Octants<Vec<Entity>> = Octants::default();
    for &entity in entities {
        let aabb = entity.aabb(aabb_storage);
        if x_plane.intersects_aabb(&aabb)
            || y_plane.intersects_aabb(&aabb)
            || z_plane.intersects_aabb(&aabb)
        {
            node_entities.push(entity);
        } else {
            oct_partition[octant_index(&(entity.position_aabb(aabb_storage) - point))].push(entity);
        }
    }
    (node_entities, oct_partition)
}

fn partition_children(
    entities: &[Entity],
    aabb: &Aabb,
    aabb_storage: &ReadStorage<AabbComponent>,
    child_node_max_size: usize,
) -> (Vec<Entity>, Octants<Box<Node>>) {
    let (node_entities, octants) = partition_entities(entities, &aabb.center, aabb_storage);
    let aabb_octants = aabb.partition();
    let tfl = Box::new(Node::_new_from_entities(
        &octants.tfl,
        aabb_octants.tfl,
        aabb_storage,
        child_node_max_size,
    ));
    let tfr = Box::new(Node::_new_from_entities(
        &octants.tfr,
        aabb_octants.tfr,
        aabb_storage,
        child_node_max_size,
    ));
    let tbl = Box::new(Node::_new_from_entities(
        &octants.tbl,
        aabb_octants.tbl,
        aabb_storage,
        child_node_max_size,
    ));
    let tbr = Box::new(Node::_new_from_entities(
        &octants.tbr,
        aabb_octants.tbr,
        aabb_storage,
        child_node_max_size,
    ));

    let bfl = Box::new(Node::_new_from_entities(
        &octants.bfl,
        aabb_octants.bfl,
        aabb_storage,
        child_node_max_size,
    ));
    let bfr = Box::new(Node::_new_from_entities(
        &octants.bfr,
        aabb_octants.bfr,
        aabb_storage,
        child_node_max_size,
    ));
    let bbl = Box::new(Node::_new_from_entities(
        &octants.bbl,
        aabb_octants.bbl,
        aabb_storage,
        child_node_max_size,
    ));
    let bbr = Box::new(Node::_new_from_entities(
        &octants.bbr,
        aabb_octants.bbr,
        aabb_storage,
        child_node_max_size,
    ));
    (
        node_entities,
        Octants {
            tfl,
            tfr,
            tbl,
            tbr,
            bfl,
            bfr,
            bbl,
            bbr,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{PrimitiveGeometryComponent, TransformComponent};
    use alga::general::SubsetOf;
    use rand::{distributions::Uniform, Rng};
    use specs::World;

    struct RayIntersectionParams {
        a: usize,
        tx0: f32,
        ty0: f32,
        tz0: f32,
        txm: f32,
        tym: f32,
        tzm: f32,
        tx1: f32,
        ty1: f32,
        tz1: f32,
    }

    fn ray_intersection_params(ray: &Ray, aabb: &Aabb) -> RayIntersectionParams {
        let mut a = 0;
        let mut ox = ray.origin.x;
        let mut oy = ray.origin.y;
        let mut oz = ray.origin.z;
        let mut dx = ray.direction.x;
        let mut dy = ray.direction.y;
        let mut dz = ray.direction.z;
        let min = aabb.min();
        let max = aabb.max();
        let center_x = max.x - min.x;
        let center_y = max.y - min.y;
        let center_z = max.z - min.z;

        if ray.direction.x < 0.0 {
            ox = center_x - ray.origin.x;
            dx = -ray.direction.x;
            a |= 4;
        }

        if ray.direction.y < 0.0 {
            oy = center_y - ray.origin.y;
            dy = -ray.direction.y;
            a |= 2;
        }

        if ray.direction.z < 0.0 {
            oz = center_z - ray.origin.z;
            dz = -ray.direction.z;
            a |= 1;
        }

        let tx0 = if dx != 0.0 {
            (min.x - ox) / dx
        } else if min.x - ox <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let ty0 = if dy != 0.0 {
            (min.y - oy) / dy
        } else if min.y - oy <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let tz0 = if dz != 0.0 {
            (min.z - oz) / dz
        } else if min.z - oz <= 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };

        let tx1 = if dx != 0.0 {
            (max.x - ox) / dx
        } else if max.x - ox < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let ty1 = if dy != 0.0 {
            (max.y - oy) / dy
        } else if max.y - oy < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };
        let tz1 = if dz != 0.0 {
            (max.z - oz) / dz
        } else if max.z - oz < 0.0 {
            std::f32::NEG_INFINITY
        } else {
            std::f32::INFINITY
        };

        let mut txm = 0.5 * (tx0 + tx1);
        let mut tym = 0.5 * (ty0 + ty1);
        let mut tzm = 0.5 * (tz0 + tz1);

        if txm.is_nan() {
            txm = if ray.origin.x < (min.x + max.x) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }
        if tym.is_nan() {
            tym = if ray.origin.y < (min.y + max.y) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }
        if tzm.is_nan() {
            tzm = if ray.origin.z < (min.z + max.z) / 2.0 {
                std::f32::INFINITY
            } else {
                std::f32::NEG_INFINITY
            };
        }

        RayIntersectionParams {
            a,
            tx0,
            ty0,
            tz0,
            txm,
            tym,
            tzm,
            tx1,
            ty1,
            tz1,
        }
    }

    #[test]
    fn test_first_node_x() {
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));

        // positive x
        let ray = Ray::new(Point3f::new(-10.0, -0.5, -0.5), Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 0);

        let ray = Ray::new(Point3f::new(-10.0, -0.5, 0.5), Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 1);

        let ray = Ray::new(Point3f::new(-10.0, 0.5, -0.5), Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 2);

        let ray = Ray::new(Point3f::new(-10.0, 0.5, 0.5), Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 3);

        // negative x
        let ray = Ray::new(Point3f::new(10.0, -0.5, -0.5), -Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 4);

        let ray = Ray::new(Point3f::new(10.0, -0.5, 0.5), -Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 5);

        let ray = Ray::new(Point3f::new(10.0, 0.5, -0.5), -Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 6);

        let ray = Ray::new(Point3f::new(10.0, 0.5, 0.5), -Vector3f::x());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 7);
    }

    #[test]
    fn test_first_node_y() {
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));

        // positive y
        let ray = Ray::new(Point3f::new(-0.5, -10.0, -0.5), Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 0);

        let ray = Ray::new(Point3f::new(-0.5, -10.0, 0.5), Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 1);

        let ray = Ray::new(Point3f::new(0.5, -10.0, -0.5), Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 4);

        let ray = Ray::new(Point3f::new(0.5, -10.0, 0.5), Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 5);

        // negative y
        let ray = Ray::new(Point3f::new(-0.5, 10.0, -0.5), -Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 2);

        let ray = Ray::new(Point3f::new(-0.5, 10.0, 0.5), -Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 3);

        let ray = Ray::new(Point3f::new(0.5, 10.0, -0.5), -Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 6);

        let ray = Ray::new(Point3f::new(0.5, 10.0, 0.5), -Vector3f::y());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 7);
    }

    #[test]
    fn test_first_node_z() {
        let aabb = Aabb::new_min_max(Point3f::new(-1.0, -1.0, -1.0), Point3f::new(1.0, 1.0, 1.0));

        // positive z
        let ray = Ray::new(Point3f::new(-0.5, -0.5, -10.0), Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 0);

        let ray = Ray::new(Point3f::new(-0.5, 0.5, -10.0), Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 2);

        let ray = Ray::new(Point3f::new(0.5, -0.5, -10.0), Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 4);

        let ray = Ray::new(Point3f::new(0.5, 0.5, -10.0), Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 6);

        // negative z
        let ray = Ray::new(Point3f::new(-0.5, -0.5, 10.0), -Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 1);

        let ray = Ray::new(Point3f::new(-0.5, 0.5, 10.0), -Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 3);

        let ray = Ray::new(Point3f::new(0.5, -0.5, 10.0), -Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 5);

        let ray = Ray::new(Point3f::new(0.5, 0.5, 10.0), -Vector3f::z());
        let params = ray_intersection_params(&ray, &aabb);
        let node = first_node(
            (params.tx0, params.ty0, params.tz0),
            (params.txm, params.tym, params.tzm),
        );
        assert_eq!(node ^ params.a, 7);
    }

    #[test]
    fn test_partition_entities1() {
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<AabbComponent>();
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

        let (node_entities, partition) =
            partition_entities(&entities, &Point3f::origin(), &world.read_storage());
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
        world.register::<AabbComponent>();
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
        );
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
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();
        world.register::<AabbComponent>();

        let mut entities = vec![];
        for x in -2..=2 {
            for y in -2..=2 {
                for z in -2..=2 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
            }
        }

        let bvh = Node::_new_from_entities(
            &entities,
            Aabb::merge_aabbs(
                &entities
                    .iter()
                    .map(|e| *e.aabb(&world.read_storage()))
                    .collect::<Vec<Aabb>>(),
            ),
            &world.read_storage(),
            2,
        );
        let ray = Ray::new(Point3f::new(1.0, 0.0, 10.0), -Vector3f::z());
        let (_, entity) = bvh._intersect_entity(&ray, &world.read_storage()).unwrap();
        assert!(ray
            .intersect_entity(entity, &world.read_storage())
            .is_some());
        assert!(entity
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, 0.0, 2.0)));

        let ray = Ray::new(Point3f::new(1.0, 1.0, 1.0), Vector3f::new(1.2, -2.3, 4.5));
        let (_, entity) = bvh._intersect_entity(&ray, &world.read_storage()).unwrap();
        assert!(entity
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(1.0, 1.0, 1.0)));
    }

    #[test]
    fn test_intersect2() {
        let mut world = World::new();
        world.register::<TransformComponent>();
        world.register::<PrimitiveGeometryComponent>();
        world.register::<AabbComponent>();

        let mut entities = vec![];
        for x in -4..=-2 {
            for y in -4..=-2 {
                for z in -4..=-2 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
                for z in 2..=4 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
            }
            for y in 2..=4 {
                for z in -4..=-2 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
                for z in 2..=4 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
            }
        }
        for x in 2..=4 {
            for y in -4..=-2 {
                for z in -4..=-2 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
                for z in 2..=4 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
            }
            for y in 2..=4 {
                for z in -4..=-2 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
                for z in 2..=4 {
                    entities.push(Entity::new_unitcube_w(
                        Translation3::from(Vector3f::new(x as f32, y as f32, z as f32))
                            .to_superset(),
                        &world,
                    ));
                }
            }
        }

        let aabb = Aabb::merge_aabbs(
            &entities
                .iter()
                .map(|e| *e.aabb(&world.read_storage()))
                .collect::<Vec<Aabb>>(),
        );
        let bvh = Node::_new_from_entities(&entities, aabb, &world.read_storage(), 8);
        let entity = bvh._intersect_entity(
            &Ray::new(
                Point3f::new(1.0, 0.0, 10.0),
                -Vector3f::z_axis().into_inner(),
            ),
            &world.read_storage(),
        );
        assert_eq!(entity, None);

        let entity = bvh._intersect_entity(
            &Ray::new(Point3f::origin(), Vector3f::new(1.0, 1.0, 1.0)),
            &world.read_storage(),
        );
        println!("{:?}", entity.unwrap().1.position(&world.read_storage()));
        assert!(entity
            .unwrap()
            .1
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(2.0, 2.0, 2.0)));

        let ray = Ray::new(
            Point3f::new(10.0, 10.0, 10.0),
            Vector3f::new(-1.0, -1.0, -1.0),
        );
        let entity = bvh._intersect_entity(&ray, &world.read_storage());
        assert!(entity
            .unwrap()
            .1
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(4.0, 4.0, 4.0)));

        let ray = Ray::new(Point3f::new(3.0, -10.0, 2.0), Vector3f::new(0.0, 1.0, 0.0));
        let entity = bvh._intersect_entity(&ray, &world.read_storage());
        println!("{:#?}", entity.unwrap().1.position(&world.read_storage()));
        assert!(entity
            .unwrap()
            .1
            .position(&world.read_storage())
            .almost_eq(&Point3f::new(3.0, -4.0, 2.0)));
    }

    #[test]
    fn test_random_intersect() {
        let mut rng = rand::thread_rng();
        let aabb_size = 10.0;
        let uniform_dist = Uniform::new(-aabb_size, aabb_size);
        let direction_dist = Uniform::new(-1.0, 1.0);

        for _ in 0..10 {
            let mut world = World::new();
            world.register::<PrimitiveGeometryComponent>();
            world.register::<TransformComponent>();
            world.register::<AabbComponent>();

            let mut entities: Vec<Entity> = vec![];
            for _ in 0..1000 {
                let x = rng.sample(uniform_dist);
                let y = rng.sample(uniform_dist);
                let z = rng.sample(uniform_dist);
                entities.push(Entity::new_unitcube_w(
                    Translation3::from(Vector3f::new(x, y, z)).to_superset(),
                    &world,
                ));
            }

            let octree = Node::new_from_entities(
                &entities,
                Aabb::new_min_max(
                    Point3f::new(-aabb_size, -aabb_size, -aabb_size),
                    Point3f::new(aabb_size, aabb_size, aabb_size),
                ),
                &world.read_storage(),
            );

            let ray = Ray::new(
                Point3f::new(
                    rng.sample(uniform_dist),
                    rng.sample(uniform_dist),
                    rng.sample(uniform_dist),
                ),
                Vector3f::new(
                    rng.sample(direction_dist),
                    rng.sample(direction_dist),
                    rng.sample(direction_dist),
                ),
            );

            assert_eq!(
                dbg!(ray
                    .closest_entity(&entities, &world.read_storage())
                    .map(|x| x.1)),
                dbg!(octree.intersect_entity(&ray, &world.read_storage()))
            );
        }
    }
}
