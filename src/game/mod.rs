use bevy::{app::PluginGroupBuilder, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;
pub struct GamePlugins;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {}

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
            .add(GamePlugin::default())
            .add(InputManagerPlugin::<PlayerAction>::default());
        #[cfg(debug_assertions)]
        {
            group = group.add(RapierDebugRenderPlugin::default());
        }
        group
    }
}
#[derive(Component)]
pub struct Player {}

impl Default for Player {
    fn default() -> Self {
        Self {}
    }
}

pub struct GamePlugin {}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_system)
            .add_systems(Startup, spawn_player_system);
    }
}

impl Default for GamePlugin {
    fn default() -> Self {
        Self {}
    }
}

pub fn spawn_world(mut commands: Commands) {
    commands.insert_resource(RapierConfiguration {
        gravity: Vec2::ZERO,
        ..default()
    });
}

pub fn spawn_camera_system(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

pub fn spawn_player_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let material = materials.add(ColorMaterial::from(Color::BLUE));
    let circle = meshes.add(shape::Circle::new(32.).into());
    commands
        .spawn(Player::default())
        .insert(Collider::ball(32.))
        .insert(Velocity::linear(Vec2::new(0., 1.)))
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(RigidBody::KinematicVelocityBased)
        .insert(MaterialMesh2dBundle {
            mesh: circle.into(),
            material: material,
            ..Default::default()
        });
}
