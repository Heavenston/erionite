use godot::{prelude::*, engine::{ENetMultiplayerPeer, global::Error as GError, multiplayer_api::{self, RPCMode}, multiplayer_peer::TransferMode, MultiplayerApi}};

use crate::singletones::GetSingletonEx;


#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct LobbyManager {
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl LobbyManager {
    fn multi(&self) -> Gd<MultiplayerApi> {
        self.base.get_multiplayer().unwrap()
    }

    fn setup(&mut self) {
        self.base.rpc_config("load_game".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_AUTHORITY,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": true,
            "channel": 0,
        }));

        self.multi().connect(
            "peer_connected".into(),
            Callable::from_object_method(
                &self.base, "on_player_joined",
            ),
        );
        self.multi().connect(
            "connected_to_server".into(),
            Callable::from_object_method(
                &self.base, "on_connected_to_server",
            ),
        );
    }

    #[func]
    pub fn start_lobby(&mut self, port: i32) -> GError {
        godot_print!("Starting lobby on port {port}");

        let mut peer = ENetMultiplayerPeer::new();
        let e = peer.create_server_ex(port)
            .max_clients(32)
            .done();
        if e != GError::OK {
            return e;
        }

        self.multi().set_multiplayer_peer(peer.upcast());
        let scene = "res://scenes/planet_scene.tscn";
        godot_print!("Lobby started, switching to scene {scene}");
        self.base.get_my_root().bind_mut()
            .switch_scene_multiplayer(scene.into());
        
        GError::OK
    }

    #[func]
    pub fn join_lobby(&mut self, addr: GString, port: i32) -> GError {
        let mut peer = ENetMultiplayerPeer::new();
        let e = peer.create_client_ex(addr, port)
            .done();
        if e != GError::OK {
            return e;
        }

        self.multi().set_multiplayer_peer(peer.upcast());
        
        GError::OK
    }

    #[func]
    fn on_player_joined(&mut self, id: i32) {
        if !self.multi().is_server() {
            return;
        }

        godot_print!("New {id}");
    }

    #[func]
    fn on_connected_to_server(&mut self) {
        godot_print!("Connected to server");
        self.base.get_my_root().bind_mut().remove_scenes();
    }
}

#[godot_api]
impl INode for LobbyManager {
    fn ready(&mut self) {
        self.setup();
    }
}
