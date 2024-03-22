use std::collections::HashSet;

use godot::{prelude::*, engine::{MultiplayerApi, multiplayer_api::RPCMode, multiplayer_peer::TransferMode, node::InternalMode, MultiplayerSpawner}};

use crate::{player::Player, my_multi_spawner::{MyMultiSpawner, SpawnData}};

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct PlayerSpawer {
    #[export(file = "*.tscn")]
    player_scene: GString,
    #[export]
    player_initial_follow: Option<Gd<Node3D>>,
    #[export]
    multiplayer_spawner: Option<Gd<MyMultiSpawner>>,
    #[export]
    player_relative_start_pos: Vector3,

    waiting: HashSet<i32>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl PlayerSpawer {
    fn multi(&self) -> Gd<MultiplayerApi> {
        self.base.get_multiplayer().unwrap()
    }

    fn setup(&mut self) {
        self.base.rpc_config("spawn_request".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_ANY_PEER,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": false,
            "channel": 0,
        }));

        self.multi().connect(
            "peer_connected".into(),
            Callable::from_object_method(
                &self.base, "on_player_joined",
            ),
        );
    }

    #[func]
    fn on_player_joined(&mut self, id: i32) {
        if !self.multi().is_server() {
            return;
        }

        self.waiting.insert(id);
    }

    fn spawn_request_rpc(&mut self) {
        self.base.rpc("spawn_request".into(), &[]);
    }
    
    #[func]
    fn spawn_request(&mut self) {
        let id = self.multi().get_remote_sender_id();

        godot_print!("spawn request from {id}");

        if !self.waiting.remove(&id) {
            godot_warn!("Received request from non waiting node");
            return;
        }
        
        self.base.call_deferred("spawn_player".into(), &[
            Variant::from(id)
        ]);
    }

    #[func]
    fn spawn_player(&mut self, id: i32) {
        let data = Variant::from(SpawnData {
            scene: self.player_scene.clone(),
            calls: dict!{
                "set_name": varray![ id.to_string() ],
                "set_multiplayer_authority": varray![ id ],
            },
            spawn_metadata_args: varray![
                self.multiplayer_spawner.as_mut().unwrap()
                    .get_path(),
                self.player_initial_follow.as_mut()
                    .map(|x| x.get_path().to_variant())
                    .unwrap_or(Variant::nil()),
                self.player_relative_start_pos,
            ],

            ..Default::default()
        });
        self.multiplayer_spawner.as_mut().unwrap()
            .spawn_ex().data(data).done();
    }
}

#[godot_api]
impl INode3D for PlayerSpawer {
    fn enter_tree(&mut self) {
        self.setup();
    }

    fn ready(&mut self) {
        if self.multi().is_server() {
            self.spawn_player(1);
        }
        else {
            self.spawn_request_rpc();
        }
    }
}
