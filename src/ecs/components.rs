use crate::{
    block::BlockType,
    geometry::{PrimitiveGeometry, Rectangle, Square, UnitCube, AABB},
    types::prelude::*,
    vulkan::Vertex3f,
};
use specs::prelude::*;
use specs_derive::Component;

#[derive(Debug, Copy, Clone)]
pub struct TransformComponent(pub Transform3f);

impl Component for TransformComponent {
    type Storage = FlaggedStorage<Self>;
}

#[derive(Component, Debug, Clone)]
pub enum PrimitiveGeometryComponent {
    Rectangle(Rectangle),
    Square(Square),
    UnitCube(UnitCube),
}

impl PrimitiveGeometryComponent {
    pub fn vtx_data(&self, transform: &Transform3f) -> Vec<Vertex3f> {
        match self {
            PrimitiveGeometryComponent::Rectangle(rect) => rect.vtx_data(transform),
            PrimitiveGeometryComponent::Square(square) => square.vtx_data(transform),
            PrimitiveGeometryComponent::UnitCube(cube) => cube.vtx_data(transform),
        }
    }

    pub fn geometry(&self) -> &PrimitiveGeometry {
        use self::PrimitiveGeometryComponent::*;
        match self {
            Rectangle(ref rect) => rect,
            Square(ref square) => square,
            UnitCube(ref cube) => cube,
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct AABBComponent(pub AABB);

#[derive(Debug, Clone, Copy, Component)]
pub struct BlockTypeComponent(pub BlockType);
