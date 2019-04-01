use std::ops::{Index, IndexMut};

#[derive(Debug)]
pub struct Octants<T> {
    /// 3
    pub tfl: T,
    /// 7
    pub tfr: T,
    /// 2
    pub tbl: T,
    /// 6
    pub tbr: T,

    /// 1
    pub bfl: T,
    /// 5
    pub bfr: T,
    /// 0
    pub bbl: T,
    /// 4
    pub bbr: T,
}

impl<T: Clone> Clone for Octants<T> {
    fn clone(&self) -> Self {
        Octants {
            tfl: self.tfl.clone(),
            tfr: self.tfr.clone(),
            tbl: self.tbl.clone(),
            tbr: self.tbr.clone(),
            bfl: self.bfl.clone(),
            bfr: self.bfr.clone(),
            bbl: self.bbl.clone(),
            bbr: self.bbr.clone(),
        }
    }
}

impl<T: Copy> Copy for Octants<T> {}

impl<T: Default> Default for Octants<T> {
    fn default() -> Octants<T> {
        Octants {
            tfl: T::default(),
            tfr: T::default(),
            tbl: T::default(),
            tbr: T::default(),
            bfl: T::default(),
            bfr: T::default(),
            bbl: T::default(),
            bbr: T::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OctantIndex {
    Tfl = 3,
    Tfr = 7,
    Tbl = 2,
    Tbr = 6,
    Bfl = 1,
    Bfr = 5,
    Bbl = 0,
    Bbr = 4,
}

impl<T> Index<usize> for Octants<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        match i {
            0 => &self.bbl,
            1 => &self.bfl,
            2 => &self.tbl,
            3 => &self.tfl,
            4 => &self.bbr,
            5 => &self.bfr,
            6 => &self.tbr,
            7 => &self.tfr,
            _ => unreachable!(),
        }
    }
}

impl<T> Index<OctantIndex> for Octants<T> {
    type Output = T;

    fn index(&self, i: OctantIndex) -> &Self::Output {
        &self[i as usize]
    }
}

impl<T> IndexMut<usize> for Octants<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        match i {
            0 => &mut self.bbl,
            1 => &mut self.bfl,
            2 => &mut self.tbl,
            3 => &mut self.tfl,
            4 => &mut self.bbr,
            5 => &mut self.bfr,
            6 => &mut self.tbr,
            7 => &mut self.tfr,
            _ => unreachable!(),
        }
    }
}

impl<T> IndexMut<OctantIndex> for Octants<T> {
    fn index_mut(&mut self, i: OctantIndex) -> &mut Self::Output {
        &mut self[i as usize]
    }
}
