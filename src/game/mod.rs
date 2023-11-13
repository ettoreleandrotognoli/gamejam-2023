use bevy::{
    app::PluginGroupBuilder, ecs::system::EntityCommands, prelude::*, sprite::MaterialMesh2dBundle,
    transform::commands, window::WindowResolution,
};
use bevy_rapier2d::prelude::*;
use bevy_turborand::prelude::*;
use leafwing_input_manager::prelude::*;
use std::{f32::consts::PI, time::Duration};

const ORIGINAL_RADIUS: f32 = 32.;
pub struct GamePlugins;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Running,
    Pause,
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    Move,
    SwapScale,
    Pause,
}

fn left_keyboard_dap() -> VirtualDPad {
    VirtualDPad {
        up: KeyCode::W.into(),
        down: KeyCode::S.into(),
        left: KeyCode::A.into(),
        right: KeyCode::D.into(),
    }
}

fn right_keyboard_dap() -> VirtualDPad {
    VirtualDPad {
        up: KeyCode::Up.into(),
        down: KeyCode::Down.into(),
        left: KeyCode::Left.into(),
        right: KeyCode::Right.into(),
    }
}

fn create_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();
    input_map.insert(left_keyboard_dap(), PlayerAction::Move);
    input_map.insert(right_keyboard_dap(), PlayerAction::Move);
    input_map.insert(KeyCode::Space, PlayerAction::SwapScale);
    input_map.insert(KeyCode::Escape, PlayerAction::Pause);
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
            .add(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0))
            .add(GamePlugin::default())
            .add(InputManagerPlugin::<PlayerAction>::default())
            .add(RngPlugin::default());
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

#[derive(Component)]
pub struct MaxSpeed {
    linear: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum ObstacleKind {
    ScaleBust(bool),
    Block,
}

impl ObstacleKind {
    pub fn add_bundle<'w, 's, 'a>(&self, entity_commands: &mut EntityCommands<'w, 's, 'a>) {
        match self {
            Self::Block => (),
            Self::ScaleBust(_) => {
                entity_commands.insert((
                    CollisionGroups::new(Group::all(), Group::all()),
                    SolverGroups::new(Group::all(), Group::NONE),
                ));
            }
        }
    }

    pub fn get_color(&self) -> Color {
        match self {
            Self::Block => Color::GRAY,
            Self::ScaleBust(dir) => {
                if *dir {
                    Color::BLUE
                } else {
                    Color::RED
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Obstacle {
    kind: ObstacleKind,
}

impl Obstacle {
    pub fn create_effect(&self, commands: &mut Commands, target: Entity, scale: &Scale) {
        match self.kind {
            ObstacleKind::ScaleBust(dir) => {
                commands.spawn((
                    BustEffect {
                        target,
                        speed: scale.speed * if dir { 2. } else { -3. },
                    },
                    Temporary {
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }
            ObstacleKind::Block => (),
        };
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
            .add_state::<GameState>()
            .add_systems(Startup, spawn_world)
            .add_systems(Startup, spawn_camera_system)
            .add_systems(Startup, spawn_player_system)
            .add_systems(Update, player_move_system)
            .add_systems(Update, player_pause_system)
            .add_systems(Update, player_swap_scale_system)
            .add_systems(Update, apply_scale_system)
            .add_systems(Update, obstacle_factory_system)
            .add_systems(Update, spawn_obstacle_system)
            .add_systems(Update, despawn_out_of_view)
            .add_systems(Update, hit_obstacle_system)
            .add_systems(Update, bust_effect_system)
            .add_systems(Update, temporary_despawn_system);
    }
}

impl Default for GamePlugin {
    fn default() -> Self {
        Self {}
    }
}

pub fn spawn_world(mut commands: Commands, mut global_rng: ResMut<GlobalRng>) {
    commands.insert_resource(RapierConfiguration {
        gravity: Vec2::ZERO,
        ..default()
    });
    commands.spawn((
        ObstacleFactoryComponent {
            timer: Timer::new(Duration::from_secs(1), TimerMode::Repeating),
        },
        RngComponent::from(&mut global_rng),
    ));
}

pub fn spawn_camera_system(mut commands: Commands) {
    commands
        .spawn(Camera2dBundle::default())
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(RigidBody::KinematicVelocityBased)
        .insert(Velocity::linear(Vec2::new(0., 80.)));
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
        random: &mut RngComponent,
        camera_info: (&Transform, &Velocity),
        player_info: (&Transform),
        event: &mut EventWriter<SpawnObstacleEvent>,
    ) {
        if !self.timer.just_finished() {
            return;
        }
        let (camera_transform, camera_velocity) = camera_info;
        let camera_direction = camera_velocity.linvel.normalize_or_zero();
        let obstacle_direction = camera_direction.rotate(Vec2::from_angle(PI / 2.));
        let obstacle_middle =
            camera_transform.translation.truncate() + (camera_direction * 1080. / 2.);
        let position = obstacle_middle + obstacle_direction * random.f32_normalized() * 720. / 2.;
        let kind = match random.u8(0..=2) {
            0 => ObstacleKind::ScaleBust(random.bool()),
            1 => ObstacleKind::ScaleBust(random.bool()),
            2 => ObstacleKind::Block,
            _ => ObstacleKind::Block,
        };
        event.send(SpawnObstacleEvent {
            color: kind.get_color(),
            position: position.extend(0.),
            radius: ORIGINAL_RADIUS,
            kind,
        })
    }
}

#[derive(Event, Debug)]
pub struct SpawnObstacleEvent {
    pub color: Color,
    pub position: Vec3,
    pub radius: f32,
    pub kind: ObstacleKind,
}

pub fn obstacle_factory_system(
    time: Res<Time>,
    mut query: Query<(&mut ObstacleFactoryComponent, &mut RngComponent)>,
    mut events: EventWriter<SpawnObstacleEvent>,
    camera_query: Query<(&Transform, &Velocity), With<Camera>>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(camera_info) = camera_query.get_single() {
        if let Ok(player_info) = player_query.get_single() {
            for (mut factory, mut random) in query.iter_mut() {
                factory.tick(time.delta());
                factory.create(&mut random, camera_info, player_info, &mut events);
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
        let mut obstacle_commands = commands.spawn(Obstacle { kind: event.kind });
        obstacle_commands
            .insert(Collider::ball(event.radius))
            .insert(Sleeping::disabled())
            .insert(RigidBody::Fixed)
            //.insert(CollidingEntities::default())
            //.insert(Sensor::default())
            .insert(ActiveEvents::all())
            //.insert(ActiveHooks::all())
            .insert(MaterialMesh2dBundle {
                mesh: circle.into(),
                material: material,
                transform: Transform::from_translation(event.position),
                ..Default::default()
            });
        event.kind.add_bundle(&mut obstacle_commands);
    }
}

pub fn spawn_player_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let initial_scale_speed = 0.5;
    let initial_size = ORIGINAL_RADIUS;
    let material = materials.add(ColorMaterial::from(Color::CYAN));
    let circle = meshes.add(shape::Circle::new(initial_size).into());
    commands
        .spawn(Player::default())
        .insert(MaxSpeed { linear: 200. })
        .insert(create_input_manager())
        .insert(Scale {
            speed: initial_scale_speed,
        })
        .insert(Collider::ball(initial_size))
        .insert(Sleeping::disabled())
        .insert(Ccd::enabled())
        .insert(CollidingEntities::default())
        .insert(ActiveHooks::FILTER_CONTACT_PAIRS)
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
    query: Query<(Entity, &ActionState<PlayerAction>, &MaxSpeed), With<Player>>,
) {
    for (entity, action_state, max_speed) in query.iter() {
        let speed = max_speed.linear;
        if let Some(move_axis_pair) = action_state.axis_pair(PlayerAction::Move) {
            let direction = move_axis_pair.xy();
            let speed = direction.normalize_or_zero() * speed;
            commands.entity(entity).insert(Velocity::linear(speed));
        }
    }
}

pub fn player_pause_system(
    mut time: ResMut<Time<Virtual>>,
    query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
) {
    for (_, action_state) in query.iter() {
        if action_state.just_pressed(PlayerAction::Pause) {
            if time.is_paused() {
                time.unpause();
            } else {
                time.pause();
            }
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
    camera_query: Query<(&Transform, &Velocity), With<Camera>>,
    query: Query<(Entity, &ViewVisibility, &Transform), Without<Player>>,
) {
    let camera_info = camera_query.get_single().unwrap();
    let camera_position = camera_info.0.translation;
    let camera_dir = camera_info.1.linvel.normalize_or_zero();
    for (entity, view_visibility, transform) in query.iter() {
        if view_visibility.get() {
            continue;
        }
        let angle = (transform.translation - camera_position)
            .truncate()
            .normalize_or_zero()
            .angle_between(camera_dir);
        if angle >= PI {
            println!("despawn {:?}", entity);
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn hit_obstacle_system(
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mut player_query: Query<(Entity, &CollidingEntities, &Scale, &Transform), With<Player>>,
    obstacle_query: Query<(Entity, &Obstacle, &Transform)>,
) {
    for player_info in player_query.iter_mut() {
        let (player_entity, colliding_entities, scale, player_transform) = player_info;
        let player_length = player_transform.scale.x;
        for colliding_entity in colliding_entities.iter() {
            if let Ok(obstacle_info) = obstacle_query.get(colliding_entity) {
                let (obstacle_entity, obstacle, obstacle_transform) = obstacle_info;
                let obstacle_length = obstacle_transform.scale.x;
                let intersection = rapier_context.contact_pair(colliding_entity, player_entity);
                let contact_pair_view = intersection.unwrap();
                let deepest_contact = contact_pair_view.find_deepest_contact().unwrap();
                let penetration = deepest_contact.1.dist();
                let normal = deepest_contact.0.normal();
                if player_length >= obstacle_length {
                    if penetration.abs() >= ORIGINAL_RADIUS * 2. * obstacle_length {
                        obstacle.create_effect(&mut commands, player_entity, scale);
                        commands.entity(obstacle_entity).despawn_recursive();
                    }
                } else {
                    if penetration.abs() >= ORIGINAL_RADIUS * 2. * player_length {
                        println!("kill player");
                    }
                }
            }
        }
    }
}

pub fn bust_effect_system(
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
