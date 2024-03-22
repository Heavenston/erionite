use godot::{bind::{GodotClass, godot_api}, obj::{Base, Gd}, engine::{Node3D, INode3D, StaticBody3D, MultiplayerApi, multiplayer_api::RPCMode, multiplayer_peer::TransferMode}, builtin::{Vector3, Transform3D, EulerOrder, Variant, dict, GString, Callable}, log::godot_warn};

const UPDATE_INTERVAL: f64 = 20.;

#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct PlanetaryCenter {
    #[base]
    base: Base<Node3D>,

    last_update: f64,

    #[export]
    #[init(default = 0.01)]
    speed: f64,

    #[export]
    #[init(default = Vector3::UP)]
    rotation_axis: Vector3,
}

#[godot_api]
impl PlanetaryCenter {
    fn multi(&self) -> Gd<MultiplayerApi> {
        self.base.get_multiplayer().unwrap()
    }

    fn setup(&mut self) {
        self.base.rpc_config("set_rotation".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_AUTHORITY,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": false,
            "channel": 0,
        }));
        self.base.rpc_config("sync_request".into(), Variant::from(dict! {
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

    fn set_rotation_rpc(&mut self, rot: Vector3) {
        self.base.call_deferred("rpc".into(), &[
            Variant::from(GString::from("set_rotation")),
            Variant::from(rot),
        ]);
    }

    #[func]
    fn set_rotation(&mut self, rot: Vector3) {
        self.base.set_rotation(rot);
    }

    #[func]
    fn sync_request(&mut self) {
        if !self.base.is_multiplayer_authority() {
            godot_warn!("Called sync request on non-autority");
            return;
        }

        let sender = self.multi().get_remote_sender_id();
        let rotation = self.base.get_rotation();
        self.base.rpc_id(sender.into(), "set_rotation".into(), &[
            Variant::from(rotation)
        ]);
    }
}

#[godot_api]
impl INode3D for PlanetaryCenter {
    fn ready(&mut self) {
        self.setup();
        self.base.rpc("sync_request".into(), &[]);
    }

    fn process(&mut self, delta: f64) {
        if !self.base.is_multiplayer_authority() {
            return;
        }

        self.last_update += delta;
        if self.last_update > UPDATE_INTERVAL {
            self.last_update = 0.;
            self.set_rotation_rpc(self.base.get_rotation());
        }
    }

    fn physics_process(&mut self, delta: f64) {
        self.base.rotate_object_local(
            self.rotation_axis, self.speed.to_radians() * delta
        );
    }
}
