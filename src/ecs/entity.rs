use crate::{
    ecs::{AABBComponent, PrimitiveGeometryComponent, TransformComponent},
    geometry::{UnitCube, AABB},
    types::prelude::*,
};
use specs::{world::Builder, ReadStorage, WriteStorage};
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Entity {
    pub entity: specs::Entity,
}

impl Entity {
    pub fn new_empty(entities: &specs::Entities) -> Entity {
        Entity {
            entity: entities.create(),
        }
    }

    pub fn new_clone(
        entities: &specs::Entities,
        transform_storage: &mut WriteStorage<TransformComponent>,
        geom_storage: &mut WriteStorage<PrimitiveGeometryComponent>,
        aabb_storage: &mut WriteStorage<AABBComponent>,
        entity: &Entity,
    ) -> Entity {
        let entity = Entity::new_with_transform(
            entities,
            transform_storage,
            entity.component(transform_storage).0,
        );
        entity.set_geometry(geom_storage, entity.geometry(geom_storage).clone());
        entity.set_bounding_box(
            aabb_storage,
            AABBComponent(*entity.bounding_box(aabb_storage)),
        );
        entity
    }

    pub fn new_with_transform(
        entities: &specs::Entities,
        storage: &mut WriteStorage<TransformComponent>,
        transform: Transform3f,
    ) -> Entity {
        let entity = Entity::new_empty(entities);
        entity.set_transform(storage, transform);
        entity
    }

    pub fn new_with_transform_w(world: &specs::World, transform: Transform3f) -> Entity {
        Entity::new_with_transform(&world.entities(), &mut world.write_storage(), transform)
    }

    pub fn new_unitcube(
        entities: &specs::Entities,
        transform_storage: &mut WriteStorage<TransformComponent>,
        aabb_storage: &mut WriteStorage<AABBComponent>,
        geom_storage: &mut WriteStorage<PrimitiveGeometryComponent>,
        transform: Transform3f,
    ) -> Entity {
        let unitcube = UnitCube::new(1.0);
        let aabb = AABB::new(Point3f::new(-0.5, -0.5, -0.5), Point3f::new(0.5, 0.5, 0.5))
            .transform(&transform);
        let entity = entities
            .build_entity()
            .with(TransformComponent(transform), transform_storage)
            .with(AABBComponent(aabb), aabb_storage)
            .with(PrimitiveGeometryComponent::UnitCube(unitcube), geom_storage)
            .build();
        Entity { entity }
    }

    pub fn new_unitcube_w(world: &specs::World, transform: Transform3f) -> Entity {
        Entity::new_unitcube(
            &world.entities(),
            &mut world.write_storage(),
            &mut world.write_storage(),
            &mut world.write_storage(),
            transform,
        )
    }

    pub fn component<'a, T, D>(&self, storage: &'a specs::Storage<T, D>) -> &'a T
    where
        T: specs::Component,
        D: Deref<Target = specs::storage::MaskedStorage<T>>,
    {
        storage.get(self.entity).unwrap()
    }

    pub fn transform<'a, D>(
        &self,
        storage: &'a specs::Storage<TransformComponent, D>,
    ) -> &'a Transform3f
    where
        D: Deref<Target = specs::storage::MaskedStorage<TransformComponent>>,
    {
        &self.component(storage).0
    }

    pub fn set_transform(
        &self,
        storage: &mut WriteStorage<TransformComponent>,
        transform: Transform3f,
    ) {
        if let Some(tc) = storage.get_mut(self.entity) {
            tc.0 = transform;
        } else {
            storage
                .insert(self.entity, TransformComponent(transform))
                .expect("insert() failed");
        }
    }

    /// Panics if the entity doesn't have a `PrimitiveGeometryComponent`.
    pub fn geometry<'a, D>(
        &self,
        storage: &'a specs::Storage<PrimitiveGeometryComponent, D>,
    ) -> &'a PrimitiveGeometryComponent
    where
        D: Deref<Target = specs::storage::MaskedStorage<PrimitiveGeometryComponent>>,
    {
        self.component(storage)
    }

    pub fn set_geometry(
        &self,
        storage: &mut WriteStorage<PrimitiveGeometryComponent>,
        geometry: PrimitiveGeometryComponent,
    ) {
        if let Some(pgc) = storage.get_mut(self.entity) {
            *pgc = geometry
        } else {
            storage
                .insert(self.entity, geometry)
                .expect("insert() failed");
        }
    }

    pub fn bounding_box<'a, D>(&self, storage: &'a specs::Storage<AABBComponent, D>) -> &'a AABB
    where
        D: Deref<Target = specs::storage::MaskedStorage<AABBComponent>>,
    {
        &self.component(storage).0
    }

    pub fn set_bounding_box(&self, storage: &mut WriteStorage<AABBComponent>, aabb: AABBComponent) {
        if let Some(bb) = storage.get_mut(self.entity) {
            *bb = aabb
        } else {
            storage.insert(self.entity, aabb).expect("insert() failed");
        }
    }

    pub fn position(&self, storage: &ReadStorage<TransformComponent>) -> Point3f {
        self.transform(storage).translation()
    }
}
