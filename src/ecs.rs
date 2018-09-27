use geometry::{rectangle::Rectangle, square::Square, unitcube::UnitCube};
use specs::{DenseVecStorage, Join, ReadStorage, System, SystemData, VecStorage};
use types::*;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct TransformComponent {
    transform: Matrix4f,
}

impl TransformComponent {
    pub fn new(transform: Matrix4f) -> TransformComponent {
        TransformComponent { transform }
    }
}

#[derive(Component)]
pub enum PrimitiveGeometryComponent {
    Rectangle(Rectangle),
    Square(Square),
    UnitCube(UnitCube),
}

impl PrimitiveGeometryComponent {
    pub fn new_rect(rect: Rectangle) -> PrimitiveGeometryComponent {
        PrimitiveGeometryComponent::Rectangle(rect)
    }

    pub fn new_square(square: Square) -> PrimitiveGeometryComponent {
        PrimitiveGeometryComponent::Square(square)
    }

    pub fn new_unit_cube(unit_cube: UnitCube) -> PrimitiveGeometryComponent {
        PrimitiveGeometryComponent::UnitCube(unit_cube)
    }
}

pub struct RenderSystem;

impl<'a> System<'a> for RenderSystem {
    type SystemData = (
        ReadStorage<'a, TransformComponent>,
        ReadStorage<'a, PrimitiveGeometryComponent>,
    );

    fn run(&mut self, (transform, geometry): Self::SystemData) {
        for (transform, geometry) in (&transform, &geometry).join() {}
    }
}
