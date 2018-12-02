use crate::{
    camera::{Camera, CameraAnimation},
    game::GameState,
    geometry::{
        boundingbox::BoundingBox, rectangle::Rectangle, square::Square, unitcube::UnitCube,
        PrimitiveGeometry,
    },
    gl::{shader::Program, VertexArrayObject},
    renderer::{RenderData, Renderer},
    types::*,
    utils::{f32, pt3f, quat4f, vec3f, NSEC_PER_SEC},
};
use glutin;
use glutin::VirtualKeyCode;
use specs::prelude::*;
use specs::Entity;
use specs_derive::Component;
use std::{
    cell::RefCell, collections::HashSet, f32::consts::PI, ops::DerefMut, rc::Rc, time::Duration,
};

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

#[derive(Component)]
pub enum PrimitiveGeometryComponent {
    Rectangle(Rectangle),
    Square(Square),
    UnitCube(UnitCube),
}

impl PrimitiveGeometryComponent {
    pub fn vtx_data(&mut self, transform: &Transform3f) -> Vec<Vertex3f> {
        match self {
            PrimitiveGeometryComponent::Rectangle(ref mut rect) => rect.vtx_data(transform),
            PrimitiveGeometryComponent::Square(ref mut square) => square.vtx_data(transform),
            PrimitiveGeometryComponent::UnitCube(ref mut cube) => cube.vtx_data(transform),
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
    inserted_id: ReaderId<InsertedFlag>,
    modified_id: ReaderId<ModifiedFlag>,
    inserted: BitSet,
    modified: BitSet,
}

impl BoundingBoxComponentSystem {
    pub fn new(
        inserted_id: ReaderId<InsertedFlag>,
        modified_id: ReaderId<ModifiedFlag>,
        inserted: BitSet,
        modified: BitSet,
    ) -> BoundingBoxComponentSystem {
        BoundingBoxComponentSystem {
            inserted_id,
            modified_id,
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

        transforms.populate_inserted(&mut self.inserted_id, &mut self.inserted);
        transforms.populate_modified(&mut self.modified_id, &mut self.modified);

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

pub struct RenderState {
    pub vao: VertexArrayObject,
    pub selection_vao: VertexArrayObject,
    pub crosshair_vao: VertexArrayObject,
    pub elapsed_time: Duration,
    pub frame_time_delta: Duration,
    pub pressed_keys: HashSet<glutin::VirtualKeyCode>,
    pub mouse_delta: (f64, f64),
    pub camera: Camera,
    pub shader_program: Program,
    pub crosshair_shader_program: Program,
    pub cobblestone_texture: u32,
    pub selection_texture: u32,
    pub crosshair_texture: u32,
    pub projection: Matrix4f,
    pub selected_cube: Option<Entity>,
    pub camera_animation: Option<CameraAnimation>,
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
            ref frame_time_delta,
            ref mut camera_animation,
        } = game_state;

        let d_yaw = mouse_delta.0 as f32 / 500.0;
        let d_pitch = mouse_delta.1 as f32 / 500.0;
        let frame_time_delta_f = frame_time_delta.as_nanos() as f32 / 1_000_000_000.0f32;
        let elapsed_time_f = elapsed_time.as_nanos() as f32 / NSEC_PER_SEC as f32;
        let mut camera_animation_finished = false;
        if let Some(camera_animation) = camera_animation {
            // Check if animation has expired
            if elapsed_time_f >= camera_animation.end_time() {
                camera.pos = camera_animation.end_pos;
                camera.pitch_q = camera_animation.end_pitch_q;
                camera.yaw_q = camera_animation.end_yaw_q;
                camera_animation_finished = true;
            } else {
                let (pos, yaw_q, pitch_q) = camera_animation.at(elapsed_time_f);
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
        let camera_speed = 3.0 * frame_time_delta_f;
        for keycode in pressed_keys {
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
        unsafe {
            renderer
                .draw_frame(&game_state, &RenderData { vertices }, *resized)
                .expect("draw_frame()");
        }
    }
}

// impl<'a> System<'a> for RenderSystem {
//     type SystemData = (
//         ReadStorage<'a, TransformComponent>,
//         WriteStorage<'a, PrimitiveGeometryComponent>,
//         WriteExpect<'a, RenderState>,
//     );

//     fn run(&mut self, (transform_storage, mut geometry, mut render_state): Self::SystemData) {
//         let render_state = render_state.deref_mut();
//         let RenderState {
//             ref mut vao,
//             ref mut selection_vao,
//             ref mut crosshair_vao,
//             ref elapsed_time,
//             ref frame_time_delta,
//             ref pressed_keys,
//             ref mouse_delta,
//             ref mut camera,
//             ref mut shader_program,
//             ref mut crosshair_shader_program,
//             ref cobblestone_texture,
//             ref selection_texture,
//             ref crosshair_texture,
//             ref projection,
//             ref selected_cube,
//             ref mut camera_animation,
//         } = render_state;
//         let d_yaw = mouse_delta.0 as f32 / 500.0;
//         let d_pitch = mouse_delta.1 as f32 / 500.0;
//         let frame_time_delta_f = frame_time_delta.as_nanos() as f32 / 1_000_000_000.0f32;
//         let elapsed_time_f = elapsed_time.as_nanos() as f32 / NSEC_PER_SEC as f32;
//         let mut camera_animation_finished = false;
//         if let Some(camera_animation) = camera_animation {
//             // Check if animation has expired
//             if elapsed_time_f >= camera_animation.end_time() {
//                 camera.pos = camera_animation.end_pos;
//                 camera.pitch_q = camera_animation.end_pitch_q;
//                 camera.yaw_q = camera_animation.end_yaw_q;
//                 camera_animation_finished = true;
//             } else {
//                 let (pos, yaw_q, pitch_q) = camera_animation.at(elapsed_time_f);
//                 camera.pos = pos;
//                 camera.pitch_q = pitch_q;
//                 camera.yaw_q = yaw_q;
//                 camera_animation_finished = false;
//             }
//         } else {
//             camera.rotate((-d_yaw, d_pitch));
//         }
//         if camera_animation_finished {
//             *camera_animation = None;
//         }
//         let camera_speed = 3.0 * frame_time_delta_f;
//         for keycode in pressed_keys {
//             match keycode {
//                 VirtualKeyCode::W => camera.pos += camera_speed * camera.direction().unwrap(),
//                 VirtualKeyCode::S => camera.pos -= camera_speed * camera.direction().unwrap(),
//                 VirtualKeyCode::A => {
//                     let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
//                     camera.pos -= delta;
//                 }
//                 VirtualKeyCode::D => {
//                     let delta = camera_speed * (Vector3f::cross(&camera.direction(), &camera.up()));
//                     camera.pos += delta;
//                 }
//                 _ => (),
//             }
//         }

//         let mut vtx_buf = vec![];
//         for (transform, geometry) in (&transform_storage, &mut geometry).join() {
//             vtx_buf.extend(geometry.vtx_data(&transform.transform));
//         }

//         vao.buffer_mut().set_buf(vtx_buf);
//         vao.buffer_mut().gl_bind(GlBufferUsage::DynamicDraw);

//         unsafe {
//             gl::DepthFunc(gl::LESS);
//         }

//         shader_program.set_used();
//         shader_program.set_mat4f("view", &camera.to_matrix());
//         shader_program.set_mat4f("projection", projection);

//         unsafe {
//             gl::ActiveTexture(gl::TEXTURE0);
//             gl::BindTexture(gl::TEXTURE_2D, *cobblestone_texture);
//         }
//         vao.gl_draw();

//         match selected_cube {
//             Some(cube) => {
//                 let transform = transform_storage.get(*cube).unwrap();
//                 if let Some(PrimitiveGeometryComponent::UnitCube(ref mut cube_geom)) =
//                     geometry.get_mut(*cube)
//                 {
//                     let vtx_data = cube_geom.vtx_data(&transform.transform);
//                     selection_vao.buffer_mut().set_buf(vtx_data);
//                     selection_vao
//                         .buffer_mut()
//                         .gl_bind(GlBufferUsage::DynamicDraw);
//                     unsafe {
//                         gl::DepthFunc(gl::LEQUAL);
//                         gl::ActiveTexture(gl::TEXTURE0);
//                         gl::BindTexture(gl::TEXTURE_2D, *selection_texture);
//                     }
//                     selection_vao.gl_draw();
//                 } else {
//                     unreachable!("selected cube wasn't a unit cube");
//                 }
//             }
//             None => (),
//         }

//         crosshair_shader_program.set_used();
//         unsafe {
//             gl::Clear(gl::DEPTH_BUFFER_BIT);
//             gl::ActiveTexture(gl::TEXTURE0);
//             gl::BindTexture(gl::TEXTURE_2D, *crosshair_texture);
//         }
//         crosshair_vao.gl_draw();
//     }
// }
