use types::{Matrix4f, Point3f, Vector3f};

pub struct Square {
    pub side: f32,
    pub transform: Matrix4f,
    pub vertices: Vec<Point3f>,
}

impl Square {
    pub fn new(side: f32) -> Square {
        Square {
            side,
            transform: Matrix4f::identity(),
            vertices: vec![
                Point3f::new(-side / 2.0, 0.0, -side / 2.0),
                Point3f::new(-side / 2.0, 0.0, side / 2.0),
                Point3f::new(side / 2.0, 0.0, side / 2.0),
                Point3f::new(side / 2.0, 0.0, -side / 2.0),
            ],
        }
    }

    pub fn new_with_transform(side: f32, transform: &Matrix4f) -> Square {
        let mut sq = Square::new(side);
        sq.transform(transform);
        sq
    }

    pub fn transform(&mut self, tr: &Matrix4f) {
        self.transform = tr * self.transform;
        for vtx in self.vertices.iter_mut() {
            *vtx = Point3f::from_homogeneous(tr * vtx.to_homogeneous()).unwrap();
        }
    }

    pub fn vtx_data(&self, transform: &Matrix4f) -> Vec<f32> {
        let mut data = vec![];
        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[0].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[0.0, 1.0]);
        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[1].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[0.0, 0.0]);
        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[2].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[1.0, 0.0]);

        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[0].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[0.0, 1.0]);
        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[2].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[1.0, 0.0]);
        data.extend_from_slice(
            Point3f::from_homogeneous(transform * self.vertices[3].to_homogeneous())
                .unwrap()
                .coords
                .as_slice(),
        );
        data.extend_from_slice(&[1.0, 1.0]);

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use na::geometry::Isometry;
    use na::{Rotation3, Translation3};
    use utils;

    #[test]
    fn test_transform1() {
        let mut s = Square::new(1.0);
        let t = Translation3::from_vector(Vector3f::new(0.0, 2.0, 0.0));
        s.transform(&t.to_homogeneous());
        #[rustfmt_skip]
        assert!(utils::mat4f_almost_eq(
            &s.transform,
            &Matrix4f::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 2.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0
            )
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[0],
            &Point3f::new(-0.5, 2.0, -0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[1],
            &Point3f::new(-0.5, 2.0, 0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[2],
            &Point3f::new(0.5, 2.0, 0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[3],
            &Point3f::new(0.5, 2.0, -0.5)
        ));
    }

    #[test]
    fn test_transform2() {
        let mut s = Square::new(1.0);
        let t = Isometry::from_parts(
            Translation3::from_vector(Vector3f::new(0.0, 0.0, 0.5)),
            Rotation3::from_axis_angle(&Vector3f::x_axis(), ::std::f32::consts::FRAC_PI_2),
        );
        s.transform(&t.to_homogeneous());
        assert!(utils::pt3f_almost_eq(
            &s.vertices[0],
            &Point3f::new(-0.5, 0.5, 0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[1],
            &Point3f::new(-0.5, -0.5, 0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[2],
            &Point3f::new(0.5, -0.5, 0.5)
        ));
        assert!(utils::pt3f_almost_eq(
            &s.vertices[3],
            &Point3f::new(0.5, 0.5, 0.5)
        ));
    }
}
