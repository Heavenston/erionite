use bevy_math::{DVec3, Vec3};

use crate::Vec3Ext;

pub trait IsZeroApprox {
    fn is_zero_approx(&self) -> bool;
}

impl IsZeroApprox for f32 {
    fn is_zero_approx(&self) -> bool {
        self.abs() <= f32::EPSILON
    }
}

impl IsZeroApprox for f64 {
    fn is_zero_approx(&self) -> bool {
        self.abs() <= f64::EPSILON
    }
}

impl IsZeroApprox for DVec3 {
    fn is_zero_approx(&self) -> bool {
        self.array().iter().all(IsZeroApprox::is_zero_approx)
    }
}

impl IsZeroApprox for Vec3 {
    fn is_zero_approx(&self) -> bool {
        self.array().iter().all(IsZeroApprox::is_zero_approx)
    }
}
