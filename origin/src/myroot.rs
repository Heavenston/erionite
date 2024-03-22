use godot::prelude::*;

use crate::singletones::GetSingletonEx;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct MyRoot {
    #[base]
    base: Base<Node>,

    spawned_scenes: Vec<Gd<Node>>,

    #[init(default = OnReady::manual())]
    scenes: OnReady<Gd<Node>>,
    #[init(default = OnReady::manual())]
    multiplayer_scenes: OnReady<Gd<Node>>,
}

#[godot_api]
impl MyRoot {
    #[func]
	pub fn remove_scenes(&mut self) {
        for mut c in self.spawned_scenes.drain(..) {
            c.get_parent().unwrap().remove_child(c.clone());
            c.queue_free();
        }
    }

    #[func]
    pub fn switch_scene_multiplayer(&mut self, path: GString) {
    	let scene = load::<PackedScene>(path).instantiate().unwrap();
    	self.remove_scenes();

        self.spawned_scenes.push(scene.clone());
    	self.multiplayer_scenes.add_child(scene);
    }

    #[func]
    pub fn switch_scene(&mut self, path: GString) {
    	let scene = load::<PackedScene>(path).instantiate().unwrap();
    	self.remove_scenes();

        self.spawned_scenes.push(scene.clone());
    	self.scenes.add_child(scene);
    }
}

#[godot_api]
impl INode for MyRoot {
    fn ready(&mut self) {
        self.scenes.init(
            self.base.get_node("scenes".into()).unwrap()
        );
        self.multiplayer_scenes.init(
            self.base.get_node("multiplayer_scenes".into()).unwrap()
        );

        self.switch_scene("res://scenes/menu.tscn".into());
    }
}
