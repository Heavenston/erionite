#![feature(iter_array_chunks)]

pub mod systems;
use bevy::{math::DQuat, render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues}};
use rapier::{geometry::TriMesh, na::{Point3, Quaternion, UnitQuaternion}};
pub(crate) use systems::*;

mod plugin;
pub use plugin::*;

mod resources;
pub use resources::*;

mod collider;
pub use collider::*;

mod rigid_body;
pub use rigid_body::*;

pub use rapier3d_f64 as rapier;

pub type Float = rapier::math::Real;
pub type Vector3 = bevy::math::DVec3;
pub type RapierVector3 = rapier::math::Vector<Float>;

pub trait LibConvert {
    type RapierType: LibConvert<RapierType = Self::RapierType, BevyType = Self::BevyType>;
    type BevyType: LibConvert<RapierType = Self::RapierType, BevyType = Self::BevyType>;

    fn to_rapier(&self) -> Self::RapierType;
    fn to_bevy(&self) -> Self::BevyType;
}

impl LibConvert for Vector3 {
    type RapierType = RapierVector3;
    type BevyType = Vector3;

    fn to_rapier(&self) -> RapierVector3 {
        RapierVector3::new(
            self.x,
            self.y,
            self.z,
        )
    }

    fn to_bevy(&self) -> Vector3 {
        *self
    }
}

impl LibConvert for RapierVector3 {
    type RapierType = RapierVector3;
    type BevyType = Vector3;

    fn to_rapier(&self) -> RapierVector3 {
        *self
    }

    fn to_bevy(&self) -> Vector3 {
        Vector3::new(
            self.x,
            self.y,
            self.z,
        )
    }
}

impl LibConvert for DQuat {
    type RapierType = UnitQuaternion<Float>;
    type BevyType = DQuat;

    fn to_rapier(&self) -> Self::RapierType {
        UnitQuaternion::<Float>::new_normalize(Quaternion::from_vector(
            rapier::na::Vector4::new(
                self.x,
                self.y,
                self.z,
                self.w,
            )
        ))
    }

    fn to_bevy(&self) -> Self::BevyType {
        *self
    }
}

impl LibConvert for UnitQuaternion<Float> {
    type RapierType = UnitQuaternion<Float>;
    type BevyType = DQuat;

    fn to_rapier(&self) -> Self::RapierType {
        *self
    }

    fn to_bevy(&self) -> Self::BevyType {
        DQuat::from_xyzw(
            self.coords.x,
            self.coords.y,
            self.coords.z,
            self.coords.w,
        )
    }
}

pub trait BevyMeshExt {
    fn to_trimesh(&self) -> Option<TriMesh>;
}

impl BevyMeshExt for Mesh {
    fn to_trimesh(&self) -> Option<TriMesh> {
        if self.primitive_topology() != PrimitiveTopology::TriangleList {
            return None;
        }

        let indices = self.indices()?;

        let Some(VertexAttributeValues::Float32x3(vertices)) =
            self.attribute(Mesh::ATTRIBUTE_POSITION)
        else { return None; };

        Some(TriMesh::new(
            vertices.iter().map(|&[x, y, z]| Point3::new(
                x as Float, y as Float, z as Float
            )).collect(),
            match indices {
                Indices::U16(u) => u.iter().copied().map(u32::from).array_chunks().collect(),
                Indices::U32(u) => u.iter().copied().array_chunks().collect(),
            },
        ))
    }
}
