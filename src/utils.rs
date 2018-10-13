use crate::types::{Matrix4f, Point3f, Vector3f, Vector4f};

pub fn mat4f_col_to_vec4f(m: &Matrix4f, c: usize) -> Vector4f {
    let col = m.column(c);
    Vector4f::new(col[0], col[1], col[2], col[3])
}

const FLOAT_COMPARISON_EPSILON: f32 = 1.0e-6;

pub fn f32_almost_eq(a: f32, b: f32) -> bool {
    f32::abs(a - b) <= FLOAT_COMPARISON_EPSILON
}

pub fn pt3f_almost_eq(p1: &Point3f, p2: &Point3f) -> bool {
    f32_almost_eq(p1.x, p2.x) && f32_almost_eq(p1.y, p2.y) && f32_almost_eq(p1.z, p2.z)
}

pub fn vec4f_almost_eq(v1: &Vector4f, v2: &Vector4f) -> bool {
    f32_almost_eq(v1.x, v2.x)
        && f32_almost_eq(v1.y, v2.y)
        && f32_almost_eq(v1.z, v2.z)
        && f32_almost_eq(v1.w, v2.w)
}

pub fn mat4f_almost_eq(m1: &Matrix4f, m2: &Matrix4f) -> bool {
    vec4f_almost_eq(&mat4f_col_to_vec4f(m1, 0), &mat4f_col_to_vec4f(m2, 0))
        && vec4f_almost_eq(&mat4f_col_to_vec4f(m1, 1), &mat4f_col_to_vec4f(m2, 1))
        && vec4f_almost_eq(&mat4f_col_to_vec4f(m1, 2), &mat4f_col_to_vec4f(m2, 2))
        && vec4f_almost_eq(&mat4f_col_to_vec4f(m1, 3), &mat4f_col_to_vec4f(m2, 3))
}

pub fn vec3f_from_vec4f(v: &Vector4f) -> Vector3f {
    Vector3f::new(v.x, v.y, v.z)
}

pub fn pt3f_min(p1: &Point3f, p2: &Point3f) -> Point3f {
    Point3f::new(
        f32::min(p1.x, p2.x),
        f32::min(p1.y, p2.y),
        f32::min(p1.z, p2.z),
    )
}

pub fn pt3f_max(p1: &Point3f, p2: &Point3f) -> Point3f {
    Point3f::new(
        f32::max(p1.x, p2.x),
        f32::max(p1.y, p2.y),
        f32::max(p1.z, p2.z),
    )
}
