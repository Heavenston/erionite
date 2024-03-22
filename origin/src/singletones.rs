use godot::prelude::*;
use godot::engine::{ INode };

use crate::lobby_manager::LobbyManager;
use crate::myroot::MyRoot;

pub trait GetSingletonEx {
    fn get_singleton(&self) -> Gd<Singleton>;
    fn get_lobby_manager(&self) -> Gd<LobbyManager>;
    fn get_my_root(&self) -> Gd<MyRoot>;
}

impl<T> GetSingletonEx for Gd<T>
    where T: Inherits<Node>
{
    fn get_singleton(&self) -> Gd<Singleton> {
        self.clone().upcast().get_tree().unwrap().get_root().unwrap()
            .get_node("/root/singleton".into()).unwrap()
            .cast()
    }

    fn get_lobby_manager(&self) -> Gd<LobbyManager> {
        self.clone().upcast().get_tree().unwrap().get_root().unwrap()
            .get_node("/root/lobby_manager".into()).unwrap()
            .cast()
    }

    fn get_my_root(&self) -> Gd<MyRoot> {
        self.clone().upcast().get_tree().unwrap().get_root().unwrap()
            .get_node("/root/my_root".into()).unwrap()
            .cast()
    }
}

#[derive(GodotClass)]
#[class(base=Node)]
pub struct Singleton {
    #[base]
    base: Base<Node>,
}

#[godot_api]
impl Singleton {
}

#[godot_api]
impl INode for Singleton {
    fn ready(&mut self) {
        
    }
}
