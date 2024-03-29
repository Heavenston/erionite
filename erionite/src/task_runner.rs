use std::{cell::OnceCell, sync::{atomic::AtomicBool, Arc, Mutex}};

use bevy::{app::{Plugin, Update}, ecs::{component::Component, entity::Entity, system::{CommandQueue, Commands, Query}}};

#[derive(Component)]
pub struct TaskComponent {
    done: Arc<AtomicBool>,
    data: Arc<Mutex<Option<CommandQueue>>>,
}

pub struct TaskRunnerPlugin;
impl Plugin for TaskRunnerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, run_task_system);
    }
}

pub fn spawn_task(
    commands: &mut Commands,
    f: impl FnOnce(&mut CommandQueue) -> () + Send + 'static
) {
    let done = Arc::new(AtomicBool::new(false));
    let data = Arc::new(Mutex::new(None));
    let data_ = data.clone();
    let done_ = done.clone();
    rayon::spawn(move || {
        let mut cq = CommandQueue::default();

        f(&mut cq);

        *data_.lock().unwrap() = Some(cq);
        done_.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    commands.spawn(TaskComponent {
        done,
        data,
    });
}

fn run_task_system(
    mut commands: Commands,
    tasks: Query<(Entity, &TaskComponent)>,
) {
    for (entity, t) in &tasks {
        if !t.done.load(std::sync::atomic::Ordering::Relaxed) {
            continue;
        }
        // can be none if boolean is set before lock is changed
        let Some(mut data) = t.data.lock().unwrap().take()
        else { continue; };
        commands.append(&mut data);
        commands.entity(entity).despawn();
    }
}
