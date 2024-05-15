use super::*;

use bevy::{math::DVec3, prelude::*};
use utils::{DAabb, Vec3Ext};

#[derive(Resource, derivative::Derivative)]
#[derivative(Default)]
pub struct GravityConfig {
    #[derivative(Default(value = "6.6743"))]
    pub gravity_constant: f64,
    #[derivative(Default(value = "true"))]
    pub enabled_svo: bool,
    /// The higher this value the faster the compute but the more imprecise 
    /// it will get.
    /// 0. means the svo is fully traversed (which is slower than disabling the svo)
    #[derivative(Default(value = "0.25"))]
    pub svo_skip_threshold: f64,
}

#[ouroboros::self_referencing]
pub(super) struct GravitySvoAlloc {
    pub(super) herd: bumpalo_herd::Herd,
    #[borrows(herd)]
    #[not_covariant]
    pub(super) root_cell: Option<svo::BumpCell::<'this, SvoData>>,
}

impl GravitySvoAlloc {
    pub fn build_svo<F>(&mut self, f: F)
        where F: for<'a> FnOnce(&'a bumpalo_herd::Herd) -> svo::BumpCell::<'a, SvoData>
    {
        utils::replace_with(self, |this| {
            let mut herd = this.into_heads().herd;
            herd.reset();

            GravitySvoAllocBuilder {
                herd,
                root_cell_builder: move |herd| Some(f(herd)),
            }.build()
        });
    }
}

impl Default for GravitySvoAlloc {
    fn default() -> Self {
        Self::new(default(), |_| default())
    }
}

#[derive(Resource)]
pub struct GravitySvoContext {
    pub(super) alloc: GravitySvoAlloc,
    pub(super) root_aabb: DAabb,
    pub(super) max_depth: u32,
}

impl Default for GravitySvoContext {
    fn default() -> Self {
        Self {
            alloc: default(),
            root_aabb: DAabb::new_center_size(DVec3::zero(), DVec3::splat(100_000f64)),
            max_depth: 20,
        }
    }
}

impl GravitySvoContext {
    pub fn depth(&self) -> u32 {
        self.alloc.with_root_cell(|root_cell| {
            root_cell.as_ref().map(|svo| svo.depth()).unwrap_or(0)
        })
    }

    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    pub fn root_aabb(&self) -> DAabb {
        self.root_aabb
    }
}

