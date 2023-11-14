use bevy::{prelude::*, window::WindowResolution};

use gamejam_2023::game::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                mode: AssetMode::Processed,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    resolution: WindowResolution::new(720., 1080.),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(GamePlugins);
    app.run();
}
