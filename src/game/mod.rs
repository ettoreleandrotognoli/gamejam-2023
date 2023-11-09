use bevy::prelude::*;

pub struct GamePlugin {}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system);
    }
}

impl Default for GamePlugin {
    fn default() -> Self {
        Self {
            
        }
    }
}

pub fn spawn_camera_system(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
