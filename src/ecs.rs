use crate::{
    game::GameState,
    geometry::{
        boundingbox::BoundingBox, rectangle::Rectangle, square::Square, unitcube::UnitCube,
        PrimitiveGeometry,
    },
    renderer::{RenderData, Renderer},
    types::prelude::*,
    utils::f32,
    vulkan::Vertex3f,
};
use specs::prelude::*;
use specs_derive::Component;
use std::{cell::RefCell, ops::DerefMut, rc::Rc};
use winit::VirtualKeyCode;

const FRAME_TIME_SAMPLE_INTERVAL: f32 = 0.25;

#[derive(Debug)]
pub struct TransformComponent {
    pub transform: Transform3f,
}

impl Component for TransformComponent {
    type Storage = FlaggedStorage<Self>;
}

impl TransformComponent {
    pub fn new(transform: Transform3f) -> TransformComponent {
        TransformComponent { transform }
    }
}

#[derive(Component, Debug)]
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

pub struct BoundingBoxComponent {
    pub bbox: BoundingBox,
}

impl Component for BoundingBoxComponent {
    type Storage = FlaggedStorage<Self>;
}

impl BoundingBoxComponent {
    pub fn new(bbox: BoundingBox) -> BoundingBoxComponent {
        BoundingBoxComponent { bbox }
    }
}

pub struct BoundingBoxComponentSystem {
    reader_id: ReaderId<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
}

impl BoundingBoxComponentSystem {
    pub fn new(
        reader_id: ReaderId<ComponentEvent>,
        inserted: BitSet,
        modified: BitSet,
    ) -> BoundingBoxComponentSystem {
        BoundingBoxComponentSystem {
            reader_id,
            inserted,
            modified,
        }
    }
}

impl<'a> System<'a> for BoundingBoxComponentSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, PrimitiveGeometryComponent>,
        ReadStorage<'a, TransformComponent>,
        WriteStorage<'a, BoundingBoxComponent>,
    );

    fn run(&mut self, (entities, primitives, transforms, mut bounding_boxes): Self::SystemData) {
        self.inserted.clear();
        self.modified.clear();

        let events = transforms.channel().read(&mut self.reader_id);
        for event in events {
            match event {
                ComponentEvent::Inserted(id) => {
                    self.inserted.add(*id);
                }
                ComponentEvent::Modified(id) => {
                    self.modified.add(*id);
                }
                _ => (),
            }
        }

        for (entity, primitive, transform, _) in
            (&entities, &primitives, &transforms, &self.inserted).join()
        {
            let bbox = primitive.geometry().bounding_box(&transform.transform);
            bounding_boxes
                .insert(entity, BoundingBoxComponent::new(bbox))
                .unwrap_or_else(|err| panic!("{:?}", err));
        }
    }
}

#[derive(Component, Default)]
#[storage(NullStorage)]
pub struct SelectedComponent;

pub struct RenderSystem {
    pub renderer: Rc<RefCell<Renderer>>,
}

impl<'a> System<'a> for RenderSystem {
    type SystemData = (
        ReadStorage<'a, TransformComponent>,
        WriteStorage<'a, PrimitiveGeometryComponent>,
        WriteExpect<'a, GameState>,
    );

    fn run(&mut self, (transform_storage, mut geometry, mut game_state): Self::SystemData) {
        let mut renderer = self.renderer.borrow_mut();
        let game_state = game_state.deref_mut();
        let GameState {
            ref resized,
            ref mut camera,
            ref pressed_keys,
            ref mouse_delta,
            ref elapsed_time,
            ref frame_time,
            ref mut camera_animation,
            ref mut fps_last_sampled_time,
            ref mut fps_sample,
            ref selected,
        } = game_state;

        let elapsed_time = *elapsed_time;
        let frame_time = *frame_time;

        let d_yaw = mouse_delta.0 as f32 / 500.0;
        let d_pitch = mouse_delta.1 as f32 / 500.0;
        let mut camera_animation_finished = false;
        if let Some(camera_animation) = camera_animation {
            // Check if animation has expired
            if elapsed_time >= camera_animation.end_time() {
                camera.pos = camera_animation.end_pos;
                camera.pitch_q = camera_animation.end_pitch_q;
                camera.yaw_q = camera_animation.end_yaw_q;
                camera_animation_finished = true;
            } else {
                let (pos, yaw_q, pitch_q) = camera_animation.at(elapsed_time);
                camera.pos = pos;
                camera.pitch_q = pitch_q;
                camera.yaw_q = yaw_q;
                camera_animation_finished = false;
            }
        } else {
            camera.rotate((-d_yaw, d_pitch));
        }
        if camera_animation_finished {
            *camera_animation = None;
        }
        let camera_speed = 3.0 * frame_time;
        for keycode in pressed_keys.keys() {
            match keycode {
                VirtualKeyCode::W => camera.pos += camera_speed * camera.direction().unwrap(),
                VirtualKeyCode::S => camera.pos -= camera_speed * camera.direction().unwrap(),
                VirtualKeyCode::A => {
                    let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
                    camera.pos -= delta;
                }
                VirtualKeyCode::D => {
                    let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
                    camera.pos += delta;
                }
                _ => (),
            }
        }

        let mut vertices = vec![];
        for (transform, geometry) in (&transform_storage, &mut geometry).join() {
            vertices.extend(geometry.vtx_data(&transform.transform));
        }

        let selection_vertices = if let Some(selected) = selected {
            let transform = transform_storage.get(*selected).unwrap();
            let geometry = geometry.get(*selected).unwrap();
            Some(geometry.vtx_data(&transform.transform))
        } else {
            None
        };

        if elapsed_time >= *fps_last_sampled_time + FRAME_TIME_SAMPLE_INTERVAL {
            *fps_last_sampled_time = elapsed_time;
            *fps_sample = 1.0 / frame_time;
        }

        let fps = *fps_sample;

        renderer
            .draw_frame(
                &game_state,
                &RenderData {
                    vertices,
                    fps,
                    selection_vertices,
                },
                *resized,
            )
            .expect("draw_frame()");
    }
}
