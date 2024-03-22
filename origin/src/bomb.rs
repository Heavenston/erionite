use godot::{prelude::*, engine::{RigidBody3D, IRigidBody3D, physics_server_3d::BodyAxis, MeshInstance3D, OmniLight3D, StandardMaterial3D, CpuParticles3D, GpuParticles3D, multiplayer_api::RPCMode, multiplayer_peer::TransferMode}};

use crate::voxel;

#[derive(GodotClass)]
#[class(init, base=RigidBody3D)]
struct Bomb {
    #[base]
    base: Base<RigidBody3D>,

    materials: Vec<Gd<StandardMaterial3D>>,

    /// Wether the bomb collided and is now counting towards explosion
    is_fixed: bool,
    fixed_relative_pos: Vector3,
    /// Wether the bomb exploded and is now waiting for animations to finish
    detonated: bool,

    /// Used to change color every change_interval
    next_change: f64,

    /// Initially set to detonation_delay and used to cound towards detonation
    start_detonation_delay: f64,

    collided_voxel: Option<Gd<voxel::Voxel>>,

    /// Current color from the colors array
    current_color: usize,

    /// Time from colliding to explosion
    #[export]
    detonation_delay: f64,
    /// Color change time inverval after collision
    #[export]
    change_interval: f64,
    /// Colors switched between
    #[export]
    colors: Array<Color>,
    #[export]
    mesh: Option<Gd<MeshInstance3D>>,
    #[export]
    light: Option<Gd<OmniLight3D>>,
    #[export]
    particles: Option<Gd<GpuParticles3D>>,
}

#[godot_api]
impl Bomb {
    fn me(&mut self) -> Gd<Self> {
        self.base.clone().cast()
    }

    fn setup(&mut self) {
        let me = self.me();

        self.base.connect(
            "body_entered".into(),
            Callable::from_object_method(&me, "on_body_entered"),
        );

        self.base.rpc_config("collided".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_ANY_PEER,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": true,
            "channel": 0,
        }));

        self.base.rpc_config("detonated".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_ANY_PEER,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            "call_local": true,
            "channel": 0,
        }));
    }

    fn set_color(&mut self, idx: usize) {
        self.mesh.as_mut().unwrap()
            .set_surface_override_material(0, self.materials[idx].clone().upcast());
        self.light.as_mut().unwrap()
            .set_color(self.colors.get(idx));
    }

    #[func]
    pub fn on_body_entered(&mut self, body: Gd<Node3D>) {
        if !self.base.is_multiplayer_authority() {
            return;
        }

        let Some(_) = body.get_node("voxel".into())
            .and_then(|x| x.try_cast::<voxel::Voxel>().ok())
        else { return };

        self.collided_rpc(body);
    }

    pub fn collided_rpc(&mut self, with: Gd<Node3D>) {
        self.base.call_deferred("rpc".into(), &[
            Variant::from("collided"),
            Variant::from(with.get_path()),
        ]);
    }

    #[func]
    pub fn collided(&mut self, body: NodePath) {
        let body = self.base.get_tree().unwrap().get_root().unwrap()
            .get_node(body).unwrap().cast::<Node3D>();

        // Rpc only called when this is guarenteed
        let voxel = body.get_node("voxel".into()).unwrap().cast();

        self.base.set_contact_monitor(false);

        self.base.set_axis_lock(BodyAxis::BODY_AXIS_LINEAR_X, true);
        self.base.set_axis_lock(BodyAxis::BODY_AXIS_LINEAR_Y, true);
        self.base.set_axis_lock(BodyAxis::BODY_AXIS_LINEAR_Z, true);
        self.base.set_axis_lock(BodyAxis::BODY_AXIS_ANGULAR_X, true);
        self.base.set_axis_lock(BodyAxis::BODY_AXIS_ANGULAR_Y, true);
        self.base.set_axis_lock(BodyAxis::BODY_AXIS_ANGULAR_Z, true);
        self.base.set_axis_velocity(Vector3::ZERO);
        self.base.set_angular_velocity(Vector3::ZERO);

        self.fixed_relative_pos = body.to_local(
            self.base.get_global_position()
        );
        self.is_fixed = true;
        self.collided_voxel = Some(voxel);
    }

    fn detonated_rpc(&mut self) {
        self.base.call_deferred("rpc".into(), &[
            Variant::from("detonated"),
        ]);
    }

    #[func]
    fn detonated(&mut self) {
        if let Some(prs) = &mut self.particles {
            prs.set_emitting(true);
        }
        self.detonated = true;

        self.mesh.as_mut().unwrap().hide();
        self.light.as_mut().unwrap().hide();

        if !self.base.is_multiplayer_authority() {
            return;
        }

        if let Some(vox) = &mut self.collided_voxel {
            vox.bind_mut().remove_sphere(
                self.base.get_global_position(),
                5.,
            );
        }
        else {
            godot_warn!("no voxel");
        }
    }
}

#[godot_api]
impl IRigidBody3D for Bomb {
    fn ready(&mut self) {
        self.setup();

        self.mesh.as_ref().unwrap();
        self.light.as_ref().unwrap();

        let og_mat: Gd<StandardMaterial3D> = self.mesh.as_mut().unwrap()
            .get_surface_override_material(0).unwrap().cast();
        for i in 0..self.colors.len() {
            let color = self.colors.get(i);
            let mut new_mat: Gd<StandardMaterial3D>
                = og_mat.duplicate().unwrap().cast();
            new_mat.set_albedo(color);
            new_mat.set_emission(color);
            self.materials.push(new_mat);
        }

        self.start_detonation_delay = self.detonation_delay;

        self.base.set_contact_monitor(true);
        self.base.set_max_contacts_reported(10);
    }

    fn process(&mut self, delta: f64) {
        let delta = delta as f64;

        if !self.is_fixed {
            return;
        }

        if let Some(vox) = self.collided_voxel.clone() {
            let parent = vox.get_parent().unwrap().cast::<Node3D>();
            self.base.set_global_position(
                parent.to_global(self.fixed_relative_pos)
            );
        }
        self.detonation_delay -= delta;

        if self.detonation_delay <= 0. && self.detonated {
            if let Some(prs) = &mut self.particles {
                if self.detonation_delay.abs() < 2. * prs.get_lifetime() as f64 {
                    return;
                }
            }

            if self.base.is_multiplayer_authority() {
                self.base.queue_free();
            }
        }
        if self.detonation_delay <= 0. && !self.detonated {
            if self.base.is_multiplayer_authority() {
                self.detonated_rpc();
            }
            return;
        }
        
        self.next_change += delta;

        if self.next_change >
            self.change_interval * (self.detonation_delay + 1.)
                .log(self.start_detonation_delay)
        {
            self.next_change = 0.;
            self.current_color =
                (self.current_color + 1) % self.colors.len();
            self.set_color(self.current_color);
        }
    }
}
