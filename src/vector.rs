use crate::types::prelude::*;
use std::{iter::IntoIterator, ops::{Deref, DerefMut, Index, IndexMut}};

#[derive(Debug, Clone)]
pub struct Vector2D<T> {
    pub side_len: usize,
    storage: Vec<T>,
}

impl<T: Clone> Vector2D<T> {
    pub fn new(side_len: usize, t: T) -> Vector2D<T> {
        assert_eq!(side_len % 2, 1, "side_len must be odd");
        Vector2D {
            side_len,
            storage: vec![t; side_len * side_len],
        }
    }
}

impl<T: Clone + Default> Vector2D<T> {
    pub fn new_default(side_len: usize) -> Vector2D<T> {
        Vector2D::new(side_len, T::default())
    }
}

impl<T> Index<usize> for Vector2D<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.storage[i]
    }
}

impl<T> IndexMut<usize> for Vector2D<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.storage[i]
    }
}

impl<T> Index<Vector2f> for Vector2D<T> {
    type Output = T;

    fn index(&self, i: Vector2f) -> &Self::Output {
        assert!(i.almost_is_int());
        &self[(i.x as i32, i.y as i32)]
    }
}

impl<T> IndexMut<Vector2f> for Vector2D<T> {
    fn index_mut(&mut self, i: Vector2f) -> &mut Self::Output {
        assert!(i.almost_is_int());
        &mut self[(i.x as i32, i.y as i32)]
    }
}

impl<T> Index<(i32, i32)> for Vector2D<T> {
    type Output = T;

    fn index(&self, (x, y): (i32, i32)) -> &Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        &self.storage[x + y * self.side_len]
    }
}

impl<T> IndexMut<(i32, i32)> for Vector2D<T> {
    fn index_mut(&mut self, (x, y): (i32, i32)) -> &mut Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        &mut self.storage[x + y * self.side_len]
    }
}

impl<T> Deref for Vector2D<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<T> DerefMut for Vector2D<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage
    }
}

impl<T: PartialEq> PartialEq for Vector2D<T> {
    fn eq(&self, other: &Vector2D<T>) -> bool {
        self.side_len == other.side_len && self.storage == other.storage
    }
}

impl<T: Eq> Eq for Vector2D<T> {}

#[derive(Debug, Clone)]
pub struct Vector3D<T> {
    pub side_len: usize,
    storage: Vec<T>,
}

impl<T: Clone> Vector3D<T> {
    pub fn new(side_len: usize, t: T) -> Vector3D<T> {
        assert_eq!(side_len % 2, 1, "side_len must be odd");
        Vector3D {
            side_len,
            storage: vec![t; side_len * side_len * side_len],
        }
    }
}

impl<T: Clone + Default> Vector3D<T> {
    pub fn new_default(side_len: usize) -> Vector3D<T> {
        Vector3D::new(side_len, T::default())
    }
}

impl<T> Index<usize> for Vector3D<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.storage[i]
    }
}

impl<T> IndexMut<usize> for Vector3D<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        &mut self.storage[i]
    }
}

impl<T> Index<Vector3f> for Vector3D<T> {
    type Output = T;

    fn index(&self, i: Vector3f) -> &Self::Output {
        assert!(i.almost_is_int());
        &self[(i.x as i32, i.y as i32, i.z as i32)]
    }
}

impl<T> IndexMut<Vector3f> for Vector3D<T> {
    fn index_mut(&mut self, i: Vector3f) -> &mut Self::Output {
        assert!(i.almost_is_int());
        &mut self[(i.x as i32, i.y as i32, i.z as i32)]
    }
}

impl<T> Index<(i32, i32, i32)> for Vector3D<T> {
    type Output = T;

    fn index(&self, (x, y, z): (i32, i32, i32)) -> &Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &self.storage[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl<T> IndexMut<(i32, i32, i32)> for Vector3D<T> {
    fn index_mut(&mut self, (x, y, z): (i32, i32, i32)) -> &mut Self::Output {
        let offset = (self.side_len as f32 / 2.0) as i32;
        let x = (x + offset) as usize;
        let y = (y + offset) as usize;
        let z = (z + offset) as usize;
        assert!(x < self.side_len);
        assert!(y < self.side_len);
        assert!(z < self.side_len);
        &mut self.storage[x + y * self.side_len + z * self.side_len * self.side_len]
    }
}

impl<T> Deref for Vector3D<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.storage
    }
}

impl<T> DerefMut for Vector3D<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage
    }
}

impl<T: PartialEq> PartialEq for Vector3D<T> {
    fn eq(&self, other: &Vector3D<T>) -> bool {
        self.side_len == other.side_len && self.storage == other.storage
    }
}

impl<T: Eq> Eq for Vector3D<T> {}

impl<T> IntoIterator for Vector3D<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.storage.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Vector3D<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.storage.iter()
    }
}
