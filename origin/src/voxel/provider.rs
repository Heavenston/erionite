use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use crate::generator::{Generator, TryIntoGenerator};

use crate::singletones::GetSingletonEx;
use crate::svo::{TerrainCellKind, CellPath, StatBool, StatInt};
use crate::unsafe_send::UnsafeSend;
use crate::{svo, marching_cubes, every_cubes::every_cubes};

use cached::proc_macro::cached;
use either::Either::{ self, Left, Right };
use godot::engine::character_body_3d::MotionMode;
use godot::engine::geometry_instance_3d::ShadowCastingSetting;
use godot::engine::global::Key;
use godot::engine::input::MouseMode;
use godot::engine::multiplayer_api::RPCMode;
use godot::engine::multiplayer_peer::TransferMode;
use godot::engine::{
    mesh, ConcavePolygonShape3D, CollisionShape3D, SurfaceTool, NoiseTexture3D,
    FastNoiseLite, Material, Shape3D, MultiplayerApi
};
use godot::prelude::*;
use godot::engine::{
    CharacterBody3D, ICharacterBody3D, InputEvent, InputEventMouseMotion,
    PhysicsServer3D, RigidBody3D, IRigidBody3D, CollisionPolygon3D,
    MeshInstance3D, Mesh, ArrayMesh
};
use itertools::Itertools as _;
use ordered_float::OrderedFloat;
use rand::prelude::*;
use arbitrary_int::*;
use noise::NoiseFn;
use rayon::prelude::*;

/// Responsible for voxel chunk generation and/or
/// multiplayer arbitrary depth svo requesting
/// (not chunk based)
#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct VoxelProvider {
    #[export]
    aabb: Aabb,

    #[export]
    /// Maximum subdivisions at which to generate the terrain
    /// also should be at which subdivs the terrain starts being editable
    max_total_subdivs: u32,

    #[export]
    /// The whole space is initially generated at this depth
    initial_gen_subdivs: u32,

    #[export]
    /// Maximum amount of subdivs generated at once
    gen_max_subdivs: u32,

    #[export]
    generator: Option<Gd<Resource>>,

    root_svo: svo::TerrainCell,
    pending_chunk_updates: HashMap<svo::CellPath, u32>,

    /// Peer id to subscribtions svo
    /// negative means not subscribed
    subscriptions: HashMap<i32, svo::Cell<StatInt<i64>>>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl VoxelProvider {
    fn multi(&self) -> Gd<MultiplayerApi> {
        self.base.get_multiplayer().unwrap()
    }

    fn setup(&mut self) {
        self.base.rpc_config("set_subscription".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_AUTHORITY,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": false,
            "channel": 0,
        }));

        self.base.rpc_config("recv_svo_update".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_ANY_PEER,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": false,
            "channel": 0,
        }));

        self.base.add_user_signal_ex("svo_update".into())
            .arguments(varray![
                dict! { "name": "path", "type": VariantType::PackedByteArray },
                dict! { "name": "svo", "type": VariantType::PackedByteArray },
                dict! { "name": "subdivs", "type": VariantType::Int },
            ])
            .done();
    }

    fn subscribers(&self, path: svo::CellPath) -> Vec<i32> {
        self.subscriptions.iter()
            .filter(|(_, subs)| match subs.follow_path(path).1.data() {
                Left(l) => l.max,
                Right(l) => l.0,
            } > 0)
            .map(|(id, _)| *id)
            .collect()
    }

    /// See [set_subscription](Self::set_subscription)
    pub fn set_subscription_rpc(
        &mut self, path: svo::CellPath, depth: u32, subscribe: bool
    ) {
        self.base.call_deferred("rpc".into(), &[
            Variant::from("set_subscription"),
            Variant::from(path),
            Variant::from(depth),
            Variant::from(subscribe),
        ]);
    }

    /// RPC for subscribing to every changes occuring in the given path
    /// but only up to the given depth
    /// but if subscribe is set to false this fully unsubscribes to the path
    /// ignoring the given depth
    #[func]
    fn set_subscription(
        &mut self, path: svo::CellPath, depth: u32, subscribe: bool
    ) {
        let id = self.multi().get_remote_sender_id();

        if path.depth() + depth > self.max_total_subdivs {
            godot_warn!("sub too deep (id: {id}, path: {path:?}, depth: {depth})");
            return;
        }

        let sub = self.subscriptions.entry(id).or_insert(StatInt(0).into());
        let (actual_path, cell) = sub.follow_path_mut(path);

        let already_correct = cell.data()
            // if it is an inner cell we can replace it with a leaf cell
            .map_left(|_| false)
            .map_right(|leaf| leaf.0 == i64::from(depth))
            .into_inner();
        if already_correct {
            godot_warn!("already subed (id: {id}, path: {path:?}, depth: {depth})");
            return;
        }
        
        let cell =
            cell.follow_path_and_split(path.reparent(actual_path.depth()));
        *cell = StatBool(subscribe).into();

        sub.simplify_on_path(path);
    }

    fn recv_svo_update_rpc(
        &mut self, peer_id: i32, path: Variant, svo: Variant, subdivs: u32,
    ) {
        self.base.call_deferred("rpc_id".into(), &[
            Variant::from(peer_id),
            Variant::from("recv_svo_update"),
            Variant::from(path),
            Variant::from(svo),
            Variant::from(subdivs),
        ]);
    }

    #[func]
    fn recv_svo_update(
        &mut self, path: Variant, svo: Variant, subdivs: u32
    ) {
        self.base.emit_signal("svo_update".into(), &[
            Variant::from(path),
            Variant::from(svo),
            Variant::from(subdivs),
        ]);
    }
}

#[godot_api]
impl INode3D for VoxelProvider {
    fn enter_tree(&mut self) {
        self.setup();
    }
}

