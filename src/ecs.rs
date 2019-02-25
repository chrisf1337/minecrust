pub mod entity;

use crate::{
    game::GameState,
    geometry::{PrimitiveGeometry, Ray, Rectangle, Square, UnitCube, AABB},
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

#[derive(Debug, Clone, Copy)]
pub struct AABBComponent(pub AABB);

impl Component for AABBComponent {
    type Storage = FlaggedStorage<Self>;
}

pub struct AABBComponentSystem {
    reader_id: ReaderId<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
}

impl AABBComponentSystem {
    pub fn new(
        reader_id: ReaderId<ComponentEvent>,
        inserted: BitSet,
        modified: BitSet,
    ) -> AABBComponentSystem {
        AABBComponentSystem {
            reader_id,
            inserted,
            modified,
        }
    }
}

impl<'a> System<'a> for AABBComponentSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, PrimitiveGeometryComponent>,
        ReadStorage<'a, TransformComponent>,
        WriteStorage<'a, AABBComponent>,
    );

    fn run(
        &mut self,
        (entities, geom_storage, transform_storage, mut aabb_storage): Self::SystemData,
    ) {
        self.inserted.clear();
        self.modified.clear();

        let events = transform_storage.channel().read(&mut self.reader_id);
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

        for (entity, geom, transform, _) in
            (&entities, &geom_storage, &transform_storage, &self.inserted).join()
        {
            match aabb_storage.get_mut(entity) {
                Some(_) => (),
                None => {
                    let aabb = geom.geometry().aabb(&transform.0);
                    aabb_storage
                        .insert(entity, AABBComponent(aabb))
                        .unwrap_or_else(|err| panic!("{:?}", err));
                }
            }
        }

        for (entity, geom, transform, _) in
            (&entities, &geom_storage, &transform_storage, &self.modified).join()
        {
            let aabb = geom.geometry().aabb(&transform.0);
            *aabb_storage.get_mut(entity).unwrap() = AABBComponent(aabb);
        }
    }
}

pub struct SelectionSystem;

impl<'a> System<'a> for SelectionSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, AABBComponent>,
        WriteExpect<'a, GameState>,
    );

    fn run(&mut self, (entities, aabb_storage, mut game_state): Self::SystemData) {
        let game_state = game_state.deref_mut();
        let GameState {
            ref camera,
            ref mut selected,
            ..
        } = game_state;

        let ray = Ray::new(camera.pos, camera.direction().into_inner());
        let mut new_selected: Option<(f32, Entity)> = None;
        for (entity, aabb) in (&entities, &aabb_storage).join() {
            if let Some((t, _)) = ray.intersect_aabb(&aabb.0) {
                if let Some((intersect_t, _)) = new_selected {
                    if t < intersect_t {
                        new_selected = Some((t, entity));
                    }
                } else {
                    new_selected = Some((t, entity));
                }
            }
        }
        *selected = new_selected.map(|(_, entity)| entity);
    }
}

pub struct OctreeSystem {
    reader_id: ReaderId<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
}

impl OctreeSystem {
    pub fn new(
        reader_id: ReaderId<ComponentEvent>,
        inserted: BitSet,
        modified: BitSet,
    ) -> OctreeSystem {
        OctreeSystem {
            reader_id,
            inserted,
            modified,
        }
    }
}

impl<'a> System<'a> for OctreeSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, AABBComponent>,
        WriteExpect<'a, GameState>,
    );

    fn run(&mut self, (entities, aabb_storage, mut game_state): Self::SystemData) {
        self.inserted.clear();
        self.modified.clear();

        let events = aabb_storage.channel().read(&mut self.reader_id);
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

        let octree = &mut game_state.octree;
        for (entity, aabb, _) in (&entities, &aabb_storage, &self.inserted).join() {}
    }
}

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
            ..
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
                VirtualKeyCode::W => camera.pos += camera_speed * camera.direction().into_inner(),
                VirtualKeyCode::S => camera.pos -= camera_speed * camera.direction().into_inner(),
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
            vertices.extend(geometry.vtx_data(&transform.0));
        }

        let selection_vertices = if let Some(selected) = selected {
            let transform = transform_storage.get(*selected).unwrap();
            let geometry = geometry.get(*selected).unwrap();
            Some(geometry.vtx_data(&transform.0))
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
