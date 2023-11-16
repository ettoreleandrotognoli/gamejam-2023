use bevy::prelude::*;

use gamejam_2023::game::*;

#[cfg(target_arch = "wasm32")]
fn asset_plugin() -> AssetPlugin {
    AssetPlugin {
        mode: AssetMode::Unprocessed,
        ..default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn asset_plugin() -> AssetPlugin {
    AssetPlugin {
        mode: AssetMode::Processed,
        ..default()
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(asset_plugin()).set(WindowPlugin {
        primary_window: Some(Window {
            resizable: false,
            resolution: (720., 1080.).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(GamePlugins);
    app.run();
}
