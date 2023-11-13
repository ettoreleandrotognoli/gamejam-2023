use bevy::{prelude::*, window::WindowResolution};

use gamejam_2023::game::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resizable: true,
            resolution: WindowResolution::new(720., 1080.),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(GamePlugins);
    app.run();
}
