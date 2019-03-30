use crate::{
    ecs::components::{
        AabbComponent, BlockComponent, PrimitiveGeometryComponent, TransformComponent,
    },
    game::GameState,
    geometry::{Ray, UnitCube, PrimitiveGeometry},
    renderer::{RenderData, Renderer},
    types::prelude::*,
    utils::f32,
};
use specs::prelude::*;
use std::{cell::RefCell, collections::HashMap, ops::DerefMut, rc::Rc};
use winit::VirtualKeyCode;

const FRAME_TIME_SAMPLE_INTERVAL: f32 = 0.25;

pub struct AabbComponentSystem {
    reader_id: ReaderId<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
}

impl AabbComponentSystem {
    pub fn new(
        reader_id: ReaderId<ComponentEvent>,
        inserted: BitSet,
        modified: BitSet,
    ) -> AabbComponentSystem {
        AabbComponentSystem {
            reader_id,
            inserted,
            modified,
        }
    }
}

impl<'a> System<'a> for AabbComponentSystem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, PrimitiveGeometryComponent>,
        ReadStorage<'a, TransformComponent>,
        WriteStorage<'a, AabbComponent>,
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
                        .insert(entity, AabbComponent(aabb))
                        .unwrap_or_else(|err| panic!("{:?}", err));
                }
            }
        }

        for (entity, geom, transform, _) in
            (&entities, &geom_storage, &transform_storage, &self.modified).join()
        {
            let aabb = geom.geometry().aabb(&transform.0);
            *aabb_storage.get_mut(entity).unwrap() = AabbComponent(aabb);
        }
    }
}

pub struct ChunkSystem {
    reader_id: ReaderId<ComponentEvent>,
    inserted: BitSet,
    modified: BitSet,
}

pub struct SelectionSystem;

impl<'a> System<'a> for SelectionSystem {
    type SystemData = (ReadStorage<'a, AabbComponent>, WriteExpect<'a, GameState>);

    fn run(&mut self, (aabb_storage, mut game_state): Self::SystemData) {
        let game_state = game_state.deref_mut();
        let GameState {
            ref camera,
            ref mut highlighted,
            ref chunk,
            ..
        } = game_state;

        let ray = Ray::new(camera.pos, camera.direction().into_inner());
        *highlighted = chunk
            .intersected_entity(&ray, &aabb_storage)
            .map(|e| e.entity);
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
        ReadStorage<'a, BlockComponent>,
    );

    fn run(
        &mut self,
        (transform_storage, mut geometry, mut game_state, block_type_storage): Self::SystemData,
    ) {
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
            ref highlighted,
            ref chunk,
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

        let mut vertices = HashMap::new();
        for (transform, geometry, block_type_component) in
            (&transform_storage, &mut geometry, &block_type_storage).join()
        {
            vertices
                .entry(block_type_component.0)
                .or_insert_with(|| vec![])
                .extend(geometry.vtx_data(&transform.0));
        }

        for (block_type, vtxs) in chunk.vtx_data(&transform_storage, &block_type_storage) {
            vertices
                .entry(block_type)
                .or_insert_with(|| vec![])
                .extend(vtxs);
        }

        let selection_vertices = if let Some(highlighted) = highlighted {
            let transform = transform_storage.get(*highlighted).unwrap();
            let cube = UnitCube::new(1.0);
            Some(cube.vtx_data(&transform.0))
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
