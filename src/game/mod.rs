use std::time::Duration;

use bevy::{app::PluginGroupBuilder, prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::*;
use leafwing_input_manager::prelude::*;
pub struct GamePlugins;

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    Move,
    SwapScale,
}

fn create_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();
    input_map.insert(
        VirtualDPad {
            up: KeyCode::W.into(),
            down: KeyCode::S.into(),
            left: KeyCode::A.into(),
            right: KeyCode::D.into(),
        },
        PlayerAction::Move,
    );
    input_map.insert(
        VirtualDPad {
            up: KeyCode::Up.into(),
            down: KeyCode::Down.into(),
            left: KeyCode::Left.into(),
            right: KeyCode::Right.into(),
        },
        PlayerAction::Move,
    );
    input_map.insert(KeyCode::Space, PlayerAction::SwapScale);
    input_map
}

fn create_input_manager() -> InputManagerBundle<PlayerAction> {
    InputManagerBundle {
        action_state: ActionState::default(),
        input_map: create_input_map(),
    }
}

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
pub struct Temporary {
    timer: Timer,
}

pub enum ObstacleKind {}

#[derive(Component)]
pub struct Obstacle {}

impl Obstacle {
    pub fn crate_effect(&self, target: Entity, scale: &Scale) -> impl Bundle {
        return (
            BustEffect {
                target: target,
                speed: scale.speed * 2.,
            },
            Temporary {
                timer: Timer::new(Duration::from_secs(1), TimerMode::Once),
            },
        );
    }
}

#[derive(Component)]
pub struct BustEffect {
    pub target: Entity,
    pub speed: f32,
}

impl BustEffect {
    pub fn apply(&self, delta: Duration, transform: &mut Transform) {
        transform.scale *= 1. + (self.speed * delta.as_secs_f32());
    }
}

#[derive(Component)]
pub struct Player {}

#[derive(Component)]
pub struct Scale {
    speed: f32,
}

impl Scale {
    pub fn swap(&mut self) {
        self.speed = -self.speed;
    }

    pub fn apply(&self, delta: Duration, transform: &mut Transform) {
        transform.scale *= 1. + (self.speed * delta.as_secs_f32());
    }
}

impl Default for Player {
    fn default() -> Self {
        Self {}
    }
}

pub struct GamePlugin {}

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnObstacleEvent>()
            .add_systems(Startup, spawn_world)
            .add_systems(Startup, spawn_camera_system)
            .add_systems(Startup, spawn_player_system)
            .add_systems(Update, player_move_system)
            .add_systems(Update, player_swap_scale_system)
            .add_systems(Update, apply_scale_system)
            .add_systems(Update, obstacle_factory_system)
            .add_systems(Update, spawn_obstacle_system)
            .add_systems(Update, despawn_out_of_view)
            .add_systems(Update, hit_obstacle_system)
            .add_systems(Update, bump_effect_system)
            .add_systems(Update, temporary_despawn_system);
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
    commands.spawn(ObstacleFactoryComponent {
        timer: Timer::new(Duration::from_secs(3), TimerMode::Repeating),
    });
}

pub fn spawn_camera_system(mut commands: Commands) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(RigidBody::KinematicVelocityBased)
        .insert(Velocity::linear(Vec2::new(0., 50.)));
}

#[derive(Component)]
pub struct ObstacleFactoryComponent {
    timer: Timer,
}

impl ObstacleFactoryComponent {
    pub fn tick(&mut self, delta: Duration) {
        self.timer.tick(delta);
    }

    pub fn create(
        &mut self,
        camera_info: (&Transform, &Velocity),
        player_info: (&Transform),
        event: &mut EventWriter<SpawnObstacleEvent>,
    ) {
        let (camera_transform, camera_velocity) = camera_info;
        let camera_direction = camera_velocity.linvel.normalize_or_zero();
        let position = camera_transform.translation + (camera_direction * 100.).extend(0.);
        if !self.timer.just_finished() {
            return;
        }
        event.send(SpawnObstacleEvent {
            color: Color::RED,
            position: position,
            radius: 32.,
        })
    }
}

#[derive(Event, Debug)]
pub struct SpawnObstacleEvent {
    pub color: Color,
    pub position: Vec3,
    pub radius: f32,
}

pub fn obstacle_factory_system(
    time: Res<Time>,
    mut query: Query<(&mut ObstacleFactoryComponent)>,
    mut events: EventWriter<SpawnObstacleEvent>,
    camera_query: Query<(&Transform, &Velocity), With<Camera>>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(camera_info) = camera_query.get_single() {
        if let Ok(player_info) = player_query.get_single() {
            for (mut factory) in query.iter_mut() {
                factory.tick(time.delta());
                factory.create(camera_info, player_info, &mut events);
            }
        }
    }
}

pub fn spawn_obstacle_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut events: EventReader<SpawnObstacleEvent>,
) {
    for event in events.read() {
        let material = materials.add(ColorMaterial::from(event.color));
        let circle = meshes.add(shape::Circle::new(event.radius).into());
        commands
            .spawn(Obstacle {})
            .insert(Collider::ball(event.radius))
            .insert(Sleeping::disabled())
            .insert(RigidBody::Fixed)
            //.insert(CollidingEntities::default())
            .insert(Sensor::default())
            .insert(ActiveEvents::COLLISION_EVENTS)
            .insert(MaterialMesh2dBundle {
                mesh: circle.into(),
                material: material,
                transform: Transform::from_translation(event.position),
                ..Default::default()
            });
    }
}

pub fn spawn_player_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let initial_scale_speed = 0.25;
    let initial_size = 32.;
    let material = materials.add(ColorMaterial::from(Color::BLUE));
    let circle = meshes.add(shape::Circle::new(initial_size).into());
    commands
        .spawn(Player::default())
        .insert(create_input_manager())
        .insert(Scale {
            speed: initial_scale_speed,
        })
        .insert(Collider::ball(initial_size))
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(CollidingEntities::default())
        //.insert(RigidBody::KinematicVelocityBased)
        .insert(RigidBody::Dynamic)
        .insert(MaterialMesh2dBundle {
            mesh: circle.into(),
            material: material,
            transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
            ..Default::default()
        });
}

pub fn player_move_system(
    mut commands: Commands,
    query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
) {
    let speed = 100.;
    for (entity, action_state) in query.iter() {
        if let Some(move_axis_pair) = action_state.axis_pair(PlayerAction::Move) {
            let direction = move_axis_pair.xy();
            let speed = direction.normalize_or_zero() * speed;
            commands.entity(entity).insert(Velocity::linear(speed));
        }
    }
}

pub fn player_swap_scale_system(
    mut query: Query<(&mut Scale, &ActionState<PlayerAction>), With<Player>>,
) {
    for (mut scale, action_state) in query.iter_mut() {
        if action_state.just_pressed(PlayerAction::SwapScale) {
            scale.swap();
        }
    }
}

pub fn apply_scale_system(time: Res<Time>, mut query: Query<(&Scale, &mut Transform)>) {
    for (scale, mut transform) in query.iter_mut() {
        scale.apply(time.delta(), &mut transform);
    }
}

pub fn despawn_out_of_view(
    mut commands: Commands,
    query: Query<(Entity, &ViewVisibility), Without<Player>>,
) {
    for (entity, view_visibility) in query.iter() {
        if !view_visibility.get() {
            commands.entity(entity).despawn_recursive();
            println!("despawn {:?}", entity);
        }
    }
}

pub fn hit_obstacle_system(
    mut commands: Commands,
    mut player_query: Query<(Entity, &CollidingEntities, &Scale), With<Player>>,
    obstacle_query: Query<(Entity, &Obstacle)>,
) {
    for player_info in player_query.iter_mut() {
        let (player_entity, colliding_entities, scale) = player_info;
        for colliding_entity in colliding_entities.iter() {
            println!("{:?}", colliding_entity);
            if let Ok(obstacle_info) = obstacle_query.get(colliding_entity) {
                let (obstacle_entity, obstacle) = obstacle_info;
                let effect_bundle = obstacle.crate_effect(player_entity, scale);
                commands.spawn(effect_bundle);
                commands.entity(obstacle_entity).despawn_recursive();
            }
        }
    }
}

pub fn bump_effect_system(
    time: Res<Time>,
    mut effect_query: Query<&BustEffect>,
    mut target_query: Query<&mut Transform>,
) {
    for effect in effect_query.iter_mut() {
        if let Ok(mut transform) = target_query.get_mut(effect.target) {
            effect.apply(time.delta(), &mut transform);
        }
    }
}

pub fn temporary_despawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Temporary)>,
) {
    for (entity, mut temporary) in query.iter_mut() {
        temporary.timer.tick(time.delta());
        if temporary.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
