use godot::{prelude::*, engine::{ENetMultiplayerPeer, global::Error as GError, multiplayer_api::{self, RPCMode}, multiplayer_peer::TransferMode, MultiplayerApi, IMultiplayerSpawner, MultiplayerSpawner}};

use crate::singletones::GetSingletonEx;

#[derive(Default, Debug, Clone, GodotConvert, FromGodot, ToGodot, PartialEq)]
pub struct SpawnData {
    /// Path to the scene to instanciate
    pub scene: GString,
    /// Dictionary of properties to set
    pub calls: Dictionary,
    /// Arguments to give to the method with_spawn_metadata
    pub spawn_metadata_args: Array<Variant>,
}

#[derive(GodotClass)]
#[class(init, base=MultiplayerSpawner)]
pub struct MyMultiSpawner {
    #[base]
    base: Base<MultiplayerSpawner>,

    /// List of allowed scenes to be spawned through the spawn method
    #[export]
    pub allowed_scenes: Array<GString>,
}

#[godot_api]
impl MyMultiSpawner {
    #[func]
    fn spawn_func(&mut self, data: SpawnData) -> Gd<Node> {
        if self.allowed_scenes.iter_shared().all(|x| x != data.scene) {
            // TODO: Should we really panic ?
            panic!("Unallowed scene spawn request");
        }

        let method_name: StringName =
            "with_spawn_metadata".into();

        let scene = load::<PackedScene>(data.scene);
        let mut instance = scene.instantiate().unwrap();

        for (key, val) in data.calls.iter_shared() {
            let Ok(name) = key.try_to::<StringName>()
                .or_else(|_| key.try_to::<String>().map(Into::into))
            else {
                godot_warn!("Invalid call type (got val {key:?} instead of StringName)");
                continue;
            };
            let Ok(args) = val.try_to::<VariantArray>()
            else {
                godot_warn!("Invalid call type (got val {val:?} instead of VariantArray)");
                continue;
            };
            instance.callv(name, args);
        }

        if instance.has_method(method_name.clone()) {
            let mut resolved_args = Array::new();
            for arg in data.spawn_metadata_args.iter_shared() {
                if let Ok(path) = arg.try_to::<NodePath>() {
                    resolved_args.push(
                        self.base.get_tree()
                            .and_then(|x| x.get_root())
                            .and_then(|x| x.get_node(path))
                            .map(|x| x.to_variant())
                            .unwrap_or_else(|| {
                                godot_warn!("Could not find path {arg:?}");
                                arg
                            })
                    );
                }
                else {
                    resolved_args.push(arg);
                }
            }

            instance.callv(method_name.clone(), resolved_args);
        }
        else {
            if data.spawn_metadata_args.len() > 0 {
                godot_warn!("No method named {method_name} but spawn args were given");
            }
        }

        instance
    }
}

#[godot_api]
impl IMultiplayerSpawner for MyMultiSpawner {
    fn enter_tree(&mut self) {
        let this = self.base.clone();
        self.base.set_spawn_function(Callable::from_object_method(
            &this, "spawn_func",
        ))
    }
}
