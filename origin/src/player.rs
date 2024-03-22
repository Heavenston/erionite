use rand::prelude::*;
use godot::builtin::math::ApproxEq;
use godot::engine::character_body_3d::MotionMode;
use godot::engine::input::MouseMode;
use godot::engine::multiplayer_api::RPCMode;
use godot::engine::multiplayer_peer::TransferMode;
use godot::prelude::*;
use godot::engine::{
    CharacterBody3D, ICharacterBody3D, InputEvent, InputEventMouseMotion, PhysicsServer3D, RigidBody3D, MultiplayerApi
};

use crate::my_multi_spawner::{MyMultiSpawner, SpawnData};
use crate::singletones::{Singleton, GetSingletonEx};
use crate::voxel::Voxel;

const FOLLOW_SLEEP_DURATION: f64 = 2.;
const FOLLOW_DURATION: f64 = 4.;

#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct Player {
    #[init(default = true)]
    user_wants_gravity_snap: bool,

    is_mouse_captured: bool,

    #[init(default = Vector3::UP * -9.8)]
    gravity: Vector3,

    #[init(default = Input::singleton())]
    input: Gd<Input>,
    #[init(default = OnReady::manual())]
    body: OnReady<Gd<Node3D>>,
    #[init(default = OnReady::manual())]
    head: OnReady<Gd<Node3D>>,
    #[init(default = OnReady::manual())]
    camera: OnReady<Gd<Camera3D>>,
    multiplayer_spawner: Option<Gd<MyMultiSpawner>>,

    #[init(default = FOLLOW_SLEEP_DURATION)]
    initial_follow_sleep: f64,
    #[init(default = FOLLOW_DURATION)]
    initial_follow_duration: f64,
    follow_start_pos: Vector3,
    #[export]
    follow_relative_start_pos: Vector3,
    #[export]
    initial_follow: Option<Gd<Node3D>>,

    #[export(file = "*.tscn")]
    shoot1: GString,
    #[export]
    #[init(default = 20.)]
    shoot1_stength: f64,
    #[export(file = "*.tscn")]
    shoot2: GString,
    #[export]
    #[init(default = 20.)]
    shoot2_stength: f64,

    slave_velocity: Vector3,

    #[base]
    base: Base<CharacterBody3D>,
}

#[godot_api]
impl Player {
    fn multi(&self) -> Gd<MultiplayerApi> {
        self.base.get_multiplayer().unwrap()
    }

    fn setup(&mut self) {
        self.base.rpc_config("update_velocity".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_AUTHORITY,
            "transfer_mode": TransferMode::TRANSFER_MODE_UNRELIABLE,
            "call_local": false,
            "channel": 0,
        }));

        self.base.rpc_config("spawn_ball".into(), Variant::from(dict! {
            "rpc_mode": RPCMode::RPC_MODE_ANY_PEER,
            "transfer_mode": TransferMode::TRANSFER_MODE_RELIABLE,
            // Needed for server to spawn its own balls
            "call_local": true,
            "channel": 0,
        }));
    }

    pub fn is_running(&self) -> bool {
        self.input.is_action_pressed("move_run".into())
    }

    pub fn is_affected_by_gravity(&self) -> bool {
        self.gravity.length_squared() > 0.000001
    }

    pub fn effective_gravity_snap(&self) -> bool {
        self.user_wants_gravity_snap && self.is_affected_by_gravity()
    }

    pub fn target_speed(&self) -> f64 {
        if self.is_running() {
            self.run_target_speed()
        }
        else {
            self.walk_target_speed()
        }
    }

    pub fn walk_target_speed(&self) -> f64 {
        4.
    }

    pub fn run_target_speed(&self) -> f64 {
        8.
    }

    pub fn thrust_speed(&self) -> f64 {
        if self.is_running() {
            self.run_thrust_speed()
        }
        else {
            self.walk_thrust_speed()
        }
    }

    pub fn walk_thrust_speed(&self) -> f64 {
        8.
    }

    pub fn run_thrust_speed(&self) -> f64 {
        10.
    }

    pub fn control(&self) -> f64 {
        0.4
    }

    pub fn jump_strength(&self) -> f64 {
        5.
    }

    pub fn look_sensitivity(&self) -> f64 {
        0.005
    }

    pub fn get_input_direction(&self) -> Vector3 {
        if !self.base.is_multiplayer_authority() {
            return Vector3::ZERO;
        }

        let mut input = Vector3::new(0., 0., 0.);

        if self.input.is_action_pressed("move_forward".into()) {
            input.z -= 1.;
        }
        if self.input.is_action_pressed("move_back".into()) {
            input.z += 1.;
        }
        if self.input.is_action_pressed("move_left".into()) {
            input.x -= 1.;
        }
        if self.input.is_action_pressed("move_right".into()) {
            input.x += 1.;
        }

        input.normalized()
    }

    fn is_on_floor(&self) -> bool {
        self.user_wants_gravity_snap && self.base.is_on_floor()
    }

    fn can_jump(&self) -> bool {
        self.is_on_floor()
    }

    pub fn realign_to_gravity(&mut self) {
        if self.gravity.is_zero_approx() {
            return;
        }

        let mut trans = self.base.get_transform();

        trans.basis.set_col_b(trans.basis.col_a());
        trans.basis.set_col_a(self.gravity.normalized() * -1.);

        if trans.basis.determinant().is_zero_approx() {
            return;
        }

        trans.basis = trans.basis.orthonormalized();
        let new_a = trans.basis.col_a();
        trans.basis.set_col_a(trans.basis.col_b());
        trans.basis.set_col_b(new_a);

        self.base.set_transform(trans);
    }

    #[func]
    fn with_spawn_metadata(
        &mut self,
        multiplayer_spawner: Gd<MyMultiSpawner>,
        initial_follow_path: Variant,
        follow_relative_start_pos: Vector3,
    ) {
        self.multiplayer_spawner = Some(multiplayer_spawner);
        
        self.initial_follow = initial_follow_path.try_to().ok();
        self.follow_relative_start_pos = follow_relative_start_pos;

        if let Some(fol) = self.initial_follow.as_ref() {
            self.follow_start_pos = fol.get_global_position();
            if self.follow_relative_start_pos.is_zero_approx() {
                self.follow_relative_start_pos = fol.to_local(
                    self.base.get_position()
                );
            }
        }
    }

    #[func]
    fn update_velocity(&mut self, v: Vector3) {
        self.slave_velocity = v;
    }

    pub fn spawn_ball_rpc(&mut self, secondary: bool) {
        self.base.call_deferred("rpc_id".into(), &[
            Variant::from(1),
            Variant::from("spawn_ball"),
            Variant::from(secondary),
        ]);
    }

    #[func]
    fn spawn_ball(&mut self, secondary: bool) {
        if !self.multi().is_server() {
            return;
        }
        if self.base.get_multiplayer_authority() != self.multi().get_remote_sender_id() {
            godot_warn!("Received spawn ball from non authority");
            return;
        }

        let gt = self.head.get_global_basis() * Vector3::FORWARD;
        let data = SpawnData {
            scene: if !secondary {
                self.shoot1.clone()
            } else {
                self.shoot2.clone()
            },
            calls: dict! {
                "set_name": varray![
                    format!("ball_{}", thread_rng().gen::<u16>())
                ],
                "set_position": varray![
                    self.head.get_global_position() + gt * 1.
                ],
                "set_linear_velocity": varray![
                    gt.normalized() * if !secondary {
                        self.shoot1_stength
                    } else {
                        self.shoot2_stength
                    } + self.base.get_real_velocity()
                ],
            },

            ..Default::default()
        };
        self.multiplayer_spawner.as_mut().unwrap().spawn_ex()
            .data(Variant::from(data))
            .done();

        // let mut parent = self.base.get_parent().unwrap();

        // let scene = load::<PackedScene>(if !secondary {
        //     self.shoot1.clone()
        // } else {
        //     self.shoot2.clone()
        // });
        // let mut instance = scene.instantiate().unwrap();
        // if let Ok(mut pos) = instance.clone().try_cast::<Node3D>() {
        //     pos.set_position(self.head.get_global_position());
        // }
        // if let Ok(mut rigid) = instance.clone().try_cast::<RigidBody3D>() {
        //     let gt = self.head.get_global_basis() * Vector3::FORWARD;
        //     rigid.set_linear_velocity();
        //     rigid.add_collision_exception_with(self.base.clone().upcast());
        // }
        // instance.set_name(format!("ball_{}", thread_rng().gen::<u16>()).into());

        // parent.add_child_ex(instance).force_readable_name(true).done();
    }
}

#[godot_api]
impl ICharacterBody3D for Player {
    fn ready(&mut self) {
        self.setup();

        self.head.init(
            self.base.get_node("body/head".into())
                .expect("Where is the Head")
                .cast()
        );
        self.body.init(
            self.base.get_node("body".into())
                .expect("Where is the body")
                .cast()
        );
        self.camera.init(
            self.base.get_node("body/head/camera".into())
                .expect("Where is the camera")
                .cast()
        );

        if let Some(fol) = self.initial_follow.as_ref() {
            self.follow_start_pos = fol.get_global_position();
            if self.follow_relative_start_pos.is_zero_approx() {
                self.follow_relative_start_pos = fol.to_local(
                    self.base.get_position()
                );
            }
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // Nothing to do with inputs here
        if !self.base.is_multiplayer_authority() {
            return;
        }

        if event.is_action_pressed("escape".into()) {
            self.input.set_mouse_mode(MouseMode::MOUSE_MODE_VISIBLE);
            self.is_mouse_captured = false;
        }
        if event.is_action_pressed("move_snap".into()) {
            self.user_wants_gravity_snap = !self.user_wants_gravity_snap;
        }

        let shoot1 = event.is_action_pressed("shoot1".into());
        let shoot2 = event.is_action_pressed("shoot2".into());
        if self.is_mouse_captured && (shoot1 || shoot2) {
            self.spawn_ball_rpc(shoot2);
        }

        if event.is_action_pressed("mouse_click".into()) {
            self.input.set_mouse_mode(MouseMode::MOUSE_MODE_CAPTURED);
            self.is_mouse_captured = true;
        }

        if !self.is_mouse_captured {
            return;
        }

        if let Ok(mm) = event.try_cast::<InputEventMouseMotion>() {
            let relative = mm.get_relative() * self.look_sensitivity();
            if self.effective_gravity_snap() {
                self.body.rotate_object_local(Vector3::UP, -relative.x);
                self.head.rotate_object_local(Vector3::RIGHT, -relative.y);

                let mut r = self.head.get_rotation_degrees();
                r.x = r.x.clamp(-90., 90.);
                self.head.set_rotation_degrees(r);
            }
            else {
                self.base.rotate_object_local(Vector3::UP, -relative.x);
                self.base.rotate_object_local(Vector3::RIGHT, -relative.y);
            }
        }
    }

    fn process(&mut self, delta: f64) {
        // Nothing to do here
        if !self.base.is_multiplayer_authority() {
            return;
        }

        self.camera.set_current(true);

        self.initial_follow_duration -= delta;
        self.initial_follow_sleep -= delta;
    }

    fn physics_process(&mut self, delta: f64) {
        self.gravity = PhysicsServer3D::singleton().body_get_direct_state(
            self.base.get_rid()
        ).unwrap().get_total_gravity();

        if !self.base.is_multiplayer_authority() {
            let pos = self.base.get_global_position();
            self.base.set_velocity(self.slave_velocity);
            self.base.set_global_position(
                pos + self.slave_velocity * delta
            );
            return;
        }

        let ld = PhysicsServer3D::singleton().body_get_direct_state(
            self.base.get_rid()
        ).unwrap().get_total_linear_damp();

        if self.is_affected_by_gravity() {
            self.base.set_motion_mode(MotionMode::MOTION_MODE_GROUNDED);
            self.base.set_up_direction(self.gravity.normalized() * -1.);
        }
        else {
            self.base.set_motion_mode(MotionMode::MOTION_MODE_FLOATING);
        }

        if self.effective_gravity_snap() {
            let av = self.base.get_platform_angular_velocity();
            let plat = Basis::from_euler(EulerOrder::YXZ, av * delta);
            let n = plat * self.base.get_global_basis();
            self.base.set_global_basis(n);

            self.realign_to_gravity();
            // ;
            // self.body().get_transform().basis.rotated(, )
        }
        else {
            let look = self.head.get_global_basis();
            self.body.set_rotation(Vector3::ZERO);
            self.head.set_rotation(Vector3::ZERO);
            self.base.set_global_basis(look);
        }


        let floor = self.is_on_floor();
        let mut vel = self.base.get_velocity();

        if let Some(r) = (
            self.base.is_multiplayer_authority()
            && self.initial_follow_duration >= 0.
        ).then(|| ()).and(self.initial_follow.as_ref()) {
            let diff = FOLLOW_DURATION - self.initial_follow_duration - FOLLOW_SLEEP_DURATION;
            if self.initial_follow_sleep >= 0. {
                self.follow_start_pos = r.get_global_position();
            }
            if !diff.is_zero_approx() {
                vel = (r.get_global_position() - self.follow_start_pos) / diff;
            }
            self.base.set_global_position(
                r.to_global(self.follow_relative_start_pos)
            );
        }

        if self.base.is_multiplayer_authority() && floor {
            let trans = self.body.get_global_transform();
            let mut target_speed = self.get_input_direction().normalized()
                * self.target_speed();

            // Do not change the 'up' speed
            target_speed.y = (trans.basis.inverse() * vel).y;
            target_speed = trans.basis * target_speed;

            vel = vel.lerp(target_speed, self.control());
        }
        else if self.base.is_multiplayer_authority() {
            let thrust_direction = self.get_input_direction().normalized();
            vel += self.head.get_global_basis() * thrust_direction
                * self.thrust_speed() * delta;
        }

        if self.base.is_multiplayer_authority()
            && self.input.is_action_pressed("move_jump".into()) {
            let up = self.body.get_global_basis().col_b().normalized();
            if self.can_jump() {
                vel += up * self.jump_strength();
            }
            else {
                vel += up * self.thrust_speed() * delta;
            }
        }
        if self.base.is_multiplayer_authority()
            && self.input.is_action_pressed("move_crouch".into()) {
            let up = self.body.get_global_basis().col_b().normalized();
            vel -= up * self.thrust_speed() * delta;
        }

        vel += self.gravity * delta;
        vel -= vel * ld * delta;

        self.base.set_velocity(vel);
        self.base.move_and_slide();
        
        if !floor && self.base.is_on_floor() {
            let pv = self.base.get_platform_velocity();
            self.base.set_velocity(vel - pv);
        }

        if self.base.is_multiplayer_authority() {
            let rvel = self.base.get_real_velocity();
            self.base.rpc("update_velocity".into(), &[Variant::from(rvel)]);
        }
    }
}

