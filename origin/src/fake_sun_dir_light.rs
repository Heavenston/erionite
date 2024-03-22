use godot::{bind::{GodotClass, godot_api}, engine::{IDirectionalLight3D, DirectionalLight3D}, obj::Base};


#[derive(GodotClass)]
#[class(init, base=DirectionalLight3D)]
struct FakeSunDirLight {
    #[base]
    base: Base<DirectionalLight3D>,
}

#[godot_api]
impl IDirectionalLight3D for FakeSunDirLight {
    fn process(&mut self, _delta: f64) {
        let Some(camera) = self.base.get_viewport()
            .and_then(|v| v.get_camera_3d())
        else { return };
        let pos = camera.get_global_position();
        self.base.look_at(pos);
    }
}
