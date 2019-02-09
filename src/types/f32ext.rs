#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Positive,
    Zero,
    Negative,
}

const FLOAT_COMPARISON_EPSILON: f32 = 1.0e-6;

pub trait F32Ext {
    fn almost_eq(self, b: f32) -> bool;
    fn sign(self) -> Sign;
}

impl F32Ext for f32 {
    fn almost_eq(self, b: f32) -> bool {
        f32::abs(self - b) <= FLOAT_COMPARISON_EPSILON
    }

    fn sign(self) -> Sign {
        if self == 0.0 {
            Sign::Zero
        } else if self > 0.0 {
            Sign::Positive
        } else {
            Sign::Negative
        }
    }
}
