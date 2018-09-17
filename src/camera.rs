use na::Unit;
use std::f32;
use std::f32::consts::{FRAC_PI_2, PI};
use types::*;
use utils;

#[derive(Debug)]
pub struct Camera {
    pub pos: Point3f,
    direction: Unit<Vector3f>,
    up: Unit<Vector3f>,
    right: Unit<Vector3f>,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    /// Yaw follows the right hand rule (i.e. negative z-axis is +pi/2 yaw, while positive z-axis is
    /// -pi/2 yaw). Pitch is the same (i.e. positive y-axis is +pi/2 yaw, while negative y-axis is
    /// -pi/2 yaw).
    pub fn new(pos: Point3f, direction: Unit<Vector3f>, up: Unit<Vector3f>) -> Camera {
        let yaw = FRAC_PI_2 - f32::atan2(direction.x, -direction.z);
        let pitch = f32::asin(direction.y);
        let right = Unit::new_normalize(Vector3f::cross(&direction, &up));
        let c = Camera {
            pos,
            direction,
            up,
            right,
            yaw,
            pitch,
        };
        println!("{:#?}", c);
        c
    }

    pub fn new_with_target(pos: Point3f, target: Point3f) -> Camera {
        Camera::new(pos, Unit::new_normalize(target - pos), Vector3f::y_axis())
    }

    pub fn to_matrix(&self) -> Matrix4f {
        Matrix4f::look_at_rh(
            &self.pos,
            &(self.pos + self.direction().as_ref()),
            self.up().as_ref(),
        )
    }

    pub fn direction(&self) -> Unit<Vector3f> {
        self.direction
    }

    pub fn up(&self) -> Unit<Vector3f> {
        self.up
    }

    pub fn rotate(&mut self, (d_yaw, d_pitch): (f32, f32)) {
        self.yaw += d_yaw;
        if self.yaw > 2.0 * PI {
            self.yaw -= 2.0 * PI;
        } else if self.yaw < -2.0 * PI {
            self.yaw += 2.0 * PI;
        }

        self.pitch += d_pitch;
        if self.pitch > FRAC_PI_2 - 0.001 {
            self.pitch = FRAC_PI_2 - 0.001;
        } else if self.pitch < -FRAC_PI_2 + 0.001 {
            self.pitch = -FRAC_PI_2 + 0.001;
        }

        self.direction = Unit::new_normalize(Vector3f::new(
            f32::cos(self.yaw) * f32::cos(self.pitch),
            f32::sin(self.pitch),
            -f32::sin(self.yaw) * f32::cos(self.pitch),
        ));
        self.right = Unit::new_normalize(Vector3f::cross(&self.direction, &Vector3f::y_axis()));
        self.up = Unit::new_normalize(Vector3f::cross(&self.right, &self.direction));
    }
}
