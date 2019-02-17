use crate::{
    na::Unit,
    types::prelude::*,
    utils::{point3f, quaternion4f},
};
use std::{
    f32,
    f32::consts::{FRAC_PI_2, PI},
};

#[derive(Debug, Clone)]
pub struct Camera {
    pub pos: Point3f,
    pub pitch_q: UnitQuaternionf,
    pub yaw_q: UnitQuaternionf,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new(Point3f::origin(), -Vector3f::z_axis())
    }
}

impl Camera {
    /// Yaw is measured as rotation from (0, 0, 1). Positive yaw is in the direction of the x-axis.
    pub fn new(pos: Point3f, direction: Unit<Vector3f>) -> Camera {
        let yaw = f32::atan2(direction.x, direction.z);
        // From the right hand rule, positive pitch will depress positive z
        let pitch = f32::asin(-direction.y);
        // (pitch, yaw, roll)
        let pitch_q = UnitQuaternionf::from_euler_angles(pitch, 0.0, 0.0);
        let yaw_q = UnitQuaternionf::from_euler_angles(0.0, yaw, 0.0);
        let c = Camera {
            pos,
            pitch_q,
            yaw_q,
        };
        println!("{:#?}", c);
        c
    }

    pub fn new_with_target(pos: Point3f, target: Point3f) -> Camera {
        Camera::new(pos, Unit::new_normalize(target - pos))
    }

    pub fn to_matrix(&self) -> Matrix4f {
        Matrix4f::look_at_rh(
            &self.pos,
            &(self.pos + self.direction().as_ref()),
            self.up().as_ref(),
        )
    }

    pub fn direction(&self) -> Unit<Vector3f> {
        self.yaw_q * (self.pitch_q * Vector3f::z_axis())
    }

    pub fn up(&self) -> Unit<Vector3f> {
        Vector3f::y_axis()
    }

    pub fn rotate(&mut self, (d_yaw, d_pitch): (f32, f32)) {
        let (mut pitch, _, _) = self.pitch_q.euler_angles();
        pitch += d_pitch;
        if pitch > FRAC_PI_2 - 0.001 {
            pitch = FRAC_PI_2 - 0.001;
        } else if pitch < -FRAC_PI_2 + 0.001 {
            pitch = -FRAC_PI_2 + 0.001;
        }

        self.yaw_q = UnitQuaternionf::from_euler_angles(0.0, d_yaw, 0.0) * self.yaw_q;
        self.pitch_q = UnitQuaternionf::from_euler_angles(pitch, 0.0, 0.0);
    }

    pub fn rotate_to(&mut self, (yaw, pitch): (f32, f32)) {
        self.pitch_q = UnitQuaternionf::from_euler_angles(pitch, 0.0, 0.0);
        self.yaw_q = UnitQuaternionf::from_euler_angles(0.0, yaw, 0.0);
    }

    pub fn rotate_to_dir(&mut self, direction: &Vector3f) {
        self.rotate_to(Vector3f::z().yaw_pitch_diff(direction));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CameraAnimation {
    pub start_pos: Point3f,
    pub start_yaw_q: UnitQuaternionf,
    pub start_pitch_q: UnitQuaternionf,
    pub end_pos: Point3f,
    pub end_yaw_q: UnitQuaternionf,
    pub end_pitch_q: UnitQuaternionf,
    pub start_time: f32,
    pub duration: f32,
}

impl CameraAnimation {
    pub fn new(
        camera: &Camera,
        end_position: Point3f,
        end_direction: &Vector3f,
        start_time: f32,
        duration: f32,
    ) -> CameraAnimation {
        let (mut yaw_diff, pitch_diff) = camera.direction().yaw_pitch_diff(end_direction);
        if yaw_diff > PI {
            yaw_diff -= 2.0 * PI;
        } else if yaw_diff < -PI {
            yaw_diff += 2.0 * PI;
        }
        let rot_yaw_q = UnitQuaternionf::from_euler_angles(0.0, yaw_diff, 0.0);
        let rot_pitch_q = UnitQuaternionf::from_euler_angles(pitch_diff, 0.0, 0.0);
        CameraAnimation {
            start_pos: camera.pos,
            start_yaw_q: camera.yaw_q,
            start_pitch_q: camera.pitch_q,
            end_pos: end_position,
            end_yaw_q: rot_yaw_q * camera.yaw_q,
            end_pitch_q: rot_pitch_q * camera.pitch_q,
            start_time,
            duration,
        }
    }

    /// Returns the point, yaw quaternion, and pitch quaternion of the animation at time `t`.
    pub fn at(&self, time: f32) -> (Point3f, UnitQuaternionf, UnitQuaternionf) {
        let t = (time - self.start_time) / self.duration;
        (
            point3f::clerp(&self.start_pos, &self.end_pos, t),
            quaternion4f::clerp(&self.start_yaw_q, &self.end_yaw_q, t),
            quaternion4f::clerp(&self.start_pitch_q, &self.end_pitch_q, t),
        )
    }

    pub fn end_time(&self) -> f32 {
        self.start_time + self.duration
    }
}
