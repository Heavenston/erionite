use bevy_math::{bounding::Aabb3d, BVec3, DVec3, Vec3};
use num_traits::{Float, Num, NumAssign};

use crate::DAabb;

pub trait GlamFloat: Float + NumAssign {
    type Aabb3d: AabbExt<Self>;
    type Vec3: Vec3Ext<Self>;

    fn new(other: f64) -> Self {
        Self::from(other).unwrap()
    }
}

impl GlamFloat for f32 {
    type Aabb3d = Aabb3d;
    type Vec3 = Vec3;
}

impl GlamFloat for f64 {
    type Aabb3d = DAabb;
    type Vec3 = DVec3;
}

pub trait AabbExt<T: GlamFloat<Aabb3d = Self>>
    where Self: Copy
{
    fn min(&self) -> T::Vec3;
    fn max(&self) -> T::Vec3;

    fn size(&self) -> T::Vec3;

    fn closest_point(&self, point: T::Vec3) -> T::Vec3 {
        let point = point.array();
        let max = self.max().array();
        let min = self.min().array();

        T::Vec3::from_array([0,1,2].map(|i|
            if point[i] > max[i]      { max[i] }
            else if point[i] < min[i] { min[i] }
            else                      { point[i] }
        ))
    }
}

impl AabbExt<f32> for Aabb3d {
    fn min(&self) -> Vec3 {
        self.min
    }

    fn max(&self) -> Vec3 {
        self.max
    }

    fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

impl AabbExt<f64> for DAabb {
    fn min(&self) -> DVec3 {
        self.min()
    }

    fn max(&self) -> DVec3 {
        self.max()
    }

    fn size(&self) -> DVec3 {
        self.max() - self.min()
    }
}

pub trait Vec3Ext<T: Num + Copy>
    where Self: Copy
{
    fn new(x: T, y: T, z: T) -> Self;
    fn from_array(a: [T; 3]) -> Self {
        Self::new(a[0], a[1], a[2])
    }
    fn zero() -> Self {
        Self::new(T::zero(), T::zero(), T::zero())
    }

    fn x(&self) -> T;
    fn y(&self) -> T;
    fn z(&self) -> T;
    fn x_mut(&mut self) -> &mut T;
    fn y_mut(&mut self) -> &mut T;
    fn z_mut(&mut self) -> &mut T;

    fn array(&self) -> [T; 3];
    fn array_mut(&mut self) -> [&mut T; 3];
}

impl Vec3Ext<f32> for Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self::new(x, y, z)
    }

    fn x(&self) -> f32 {
        self.x
    }

    fn y(&self) -> f32 {
        self.y
    }

    fn z(&self) -> f32 {
        self.z
    }

    fn x_mut(&mut self) -> &mut f32 {
        &mut self.x
    }

    fn y_mut(&mut self) -> &mut f32 {
        &mut self.y
    }

    fn z_mut(&mut self) -> &mut f32 {
        &mut self.z
    }

    fn array(&self) -> [f32; 3] {
        self.to_array()
    }

    fn array_mut(&mut self) -> [&mut f32; 3] {
        [&mut self.x, &mut self.y, &mut self.z]
    }
}

impl Vec3Ext<f64> for DVec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self::new(x, y, z)
    }

    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }

    fn z(&self) -> f64 {
        self.z
    }

    fn x_mut(&mut self) -> &mut f64 {
        &mut self.x
    }

    fn y_mut(&mut self) -> &mut f64 {
        &mut self.y
    }

    fn z_mut(&mut self) -> &mut f64 {
        &mut self.z
    }

    fn array(&self) -> [f64; 3] {
        self.to_array()
    }

    fn array_mut(&mut self) -> [&mut f64; 3] {
        [&mut self.x, &mut self.y, &mut self.z]
    }
}
