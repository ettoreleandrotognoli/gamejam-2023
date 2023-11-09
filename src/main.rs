use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use gamejam_2023::game::*;

fn main() {
    App::new()
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins((DefaultPlugins, GamePlugin::default()))
        .run();
}
