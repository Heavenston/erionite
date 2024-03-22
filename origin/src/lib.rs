#![feature(int_roundings)]
#![feature(iter_array_chunks)]
#![feature(array_try_map)]
#![feature(array_methods)]
#![feature(iter_collect_into)]
#![feature(duration_constants)]
#![allow(unused_imports)]

use godot::prelude::*;

pub mod my_multi_spawner;
pub mod myroot;
pub mod player_spawer;
pub mod lobby_manager;
pub mod fake_sun_dir_light;
pub mod planetary_center;
pub mod unsafe_send;
pub mod every_cubes;
pub mod singletones;
pub mod generator;
pub mod sdf;
pub mod player;
pub mod voxel;
pub mod svo;
pub mod marching_cubes;
pub mod bomb;

struct Erionite;

#[gdextension]
unsafe impl ExtensionLibrary for Erionite {}
