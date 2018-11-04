use crate::na::Unit;
use crate::types::*;
use crate::utils::vec3f::yaw_pitch_diff;
use std::f32;
use std::f32::consts::FRAC_PI_2;

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
        println!("yaw {}  pitch {}", yaw, pitch);
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
        let (mut pitch, _, _) = self.pitch_q.to_euler_angles();
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
        self.rotate_to(yaw_pitch_diff(&Vector3f::z(), direction));
    }
}
