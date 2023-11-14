use bevy::{
    app::PluginGroupBuilder, ecs::system::EntityCommands, prelude::*, sprite::MaterialMesh2dBundle,
};
use bevy_rapier2d::prelude::*;
use bevy_turborand::{prelude::*, DelegatedRng};
use leafwing_input_manager::prelude::*;
use std::{f32::consts::PI, time::Duration};

const ORIGINAL_RADIUS: f32 = 32.;
pub struct GamePlugins;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Startup,
    Running,
    Pause,
    Over,
}

#[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum PlayerAction {
    Move,
    SwapScale,
    Pause,
    Start,
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

fn insert_gamepad(input_map: &mut InputMap<PlayerAction>) {
    input_map.insert(DualAxis::left_stick(), PlayerAction::Move);
    input_map.insert_multiple([
        (GamepadButtonType::South, PlayerAction::SwapScale),
        (GamepadButtonType::Start, PlayerAction::Pause),
        (GamepadButtonType::Start, PlayerAction::Start),
    ]);
}

fn create_input_map() -> InputMap<PlayerAction> {
    let mut input_map = InputMap::default();
    input_map.insert(left_keyboard_dap(), PlayerAction::Move);
    input_map.insert(right_keyboard_dap(), PlayerAction::Move);
    input_map.insert(KeyCode::Space, PlayerAction::SwapScale);
    input_map.insert(KeyCode::Escape, PlayerAction::Pause);
    input_map.insert(KeyCode::Escape, PlayerAction::Start);
    input_map.insert(KeyCode::Return, PlayerAction::Pause);
    input_map.insert(KeyCode::Return, PlayerAction::Start);
    insert_gamepad(&mut input_map);
    input_map
}

fn create_input_manager() -> InputManagerBundle<PlayerAction> {
    let mut input_map = create_input_map();
    input_map.set_gamepad(Gamepad { id: 0 });
    InputManagerBundle {
        action_state: ActionState::default(),
        input_map: input_map,
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

#[derive(Event)]
pub enum GameEvent {
    Start,
    GameOver,
}

#[derive(Component)]
pub struct Temporary {
    timer: Timer,
}

#[derive(Clone, Copy, Debug)]
pub enum ObstacleKind {
    ScaleBust(bool),
    Block,
    Ice,
    Poison,
}

impl ObstacleKind {
    pub fn add_bundle<'w, 's, 'a>(&self, entity_commands: &mut EntityCommands<'w, 's, 'a>) {
        match self {
            Self::Block => {
                entity_commands.insert((
                    RigidBody::Fixed,
                    CollisionGroups::new(Group::all(), Group::all()),
                    SolverGroups::new(Group::all(), Group::all()),
                ));
            }
            Self::ScaleBust(_) => {
                entity_commands.insert((
                    CollisionGroups::new(Group::all(), Group::all()),
                    SolverGroups::new(Group::from_bits_retain(0b10), Group::all()),
                    RigidBody::Dynamic,
                    Enemy::lazy_aggressive(),
                ));
            }
            Self::Ice => {
                entity_commands.insert((
                    CollisionGroups::new(Group::all(), Group::all()),
                    SolverGroups::new(Group::from_bits_retain(0b10), Group::all()),
                    RigidBody::Dynamic,
                    Enemy::lazy_suicide_aggressive(),
                ));
            }
            Self::Poison => {
                entity_commands.insert((
                    CollisionGroups::new(Group::all(), Group::all()),
                    SolverGroups::new(Group::from_bits_retain(0b10), Group::all()),
                    RigidBody::Dynamic,
                    Enemy::lazy_suicide(),
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
            Self::Ice => Color::WHITE,
            Self::Poison => Color::GREEN,
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
            ObstacleKind::Ice => {
                commands.spawn((
                    FrozenEffect { target },
                    Temporary {
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
                commands.entity(target).insert(Velocity::zero());
            }
            ObstacleKind::Poison => {
                commands.spawn(Destroy { target });
            }
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
        let new_scale = transform.scale * 1. + (self.speed * delta.as_secs_f32());
        transform.scale = Vec3::new(
            f32::min(f32::max(new_scale.x, 0.1), 20.),
            f32::min(f32::max(new_scale.y, 0.1), 20.),
            1.,
        );
    }
}

#[derive(Component)]
pub struct FrozenEffect {
    target: Entity,
}

#[derive(Component)]
pub struct Destroy {
    target: Entity,
}

pub fn calc_speed(transform: &Transform) -> f32 {
    1. / (transform.scale.truncate().length().sqrt()) * 200.
}
#[derive(Component)]
pub struct Player {}

pub enum Strategy {
    None,
    Follow { max_distance: f32 },
    Run { max_distance: f32 },
}

impl Strategy {
    pub fn calc(&self, direction: Vec2, distance: f32) -> Vec2 {
        match self {
            Self::None => Vec2::ZERO,
            Self::Follow { max_distance } => {
                if *max_distance >= distance {
                    direction
                } else {
                    Vec2::ZERO
                }
            }
            Self::Run { max_distance } => {
                if *max_distance >= distance {
                    -direction
                } else {
                    Vec2::ZERO
                }
            }
        }
    }
}

#[derive(Component)]
pub struct Enemy {
    when_bigger: Strategy,
    when_smaller: Strategy,
    when_equal: Strategy,
}

impl Enemy {
    pub fn lazy_suicide() -> Self {
        Self {
            when_bigger: Strategy::None,
            when_smaller: Strategy::Follow { max_distance: 128. },
            when_equal: Strategy::None,
        }
    }

    pub fn lazy_smart_aggressive() -> Self {
        Self {
            when_bigger: Strategy::Follow { max_distance: 128. },
            when_smaller: Strategy::Run { max_distance: 128. },
            when_equal: Strategy::None,
        }
    }

    pub fn lazy_aggressive() -> Self {
        Self {
            when_bigger: Strategy::Follow { max_distance: 128. },
            when_smaller: Strategy::None,
            when_equal: Strategy::None,
        }
    }

    pub fn lazy_suicide_aggressive() -> Self {
        Self {
            when_bigger: Strategy::Follow { max_distance: 128. },
            when_smaller: Strategy::Follow { max_distance: 128. },
            when_equal: Strategy::None,
        }
    }

    pub fn smart_aggressive() -> Self {
        Self {
            when_bigger: Strategy::Follow {
                max_distance: f32::INFINITY,
            },
            when_smaller: Strategy::Run {
                max_distance: f32::INFINITY,
            },
            when_equal: Strategy::None,
        }
    }

    pub fn tick(
        &self,
        enemy: (&Transform, &Velocity),
        player: (&Transform, &Velocity),
    ) -> Velocity {
        let enemy_length = enemy.0.scale.length();
        let player_length = player.0.scale.length();
        let player_radius = player.0.scale.length() * ORIGINAL_RADIUS;
        let diff = player.0.translation.truncate() - enemy.0.translation.truncate();
        let direction = diff.normalize_or_zero();
        let distance = f32::max(diff.length() - player_radius, 0.);

        let enemy_direction = if enemy_length > player_length {
            self.when_bigger.calc(direction, distance)
        } else if player_length > enemy_length {
            self.when_smaller.calc(direction, distance)
        } else {
            self.when_equal.calc(direction, distance)
        };

        Velocity::linear(enemy_direction * calc_speed(enemy.0))
    }
}

impl Default for Enemy {
    fn default() -> Self {
        Self {
            when_bigger: Strategy::None,
            when_smaller: Strategy::None,
            when_equal: Strategy::None,
        }
    }
}

#[derive(Component)]
pub struct TimeScore {
    elapsed_time: Duration,
}

impl Default for TimeScore {
    fn default() -> Self {
        Self {
            elapsed_time: Duration::ZERO,
        }
    }
}

impl TimeScore {
    pub fn tick(&mut self, delta: Duration) {
        self.elapsed_time += delta;
    }

    pub fn to_string(&self) -> String {
        let minutes = self.elapsed_time.as_secs() / 60;
        let seconds = self.elapsed_time.as_secs() % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[derive(Component)]
pub struct Scale {
    speed: f32,
}

impl Scale {
    pub fn swap(&mut self) {
        self.speed = -self.speed;
    }

    pub fn apply(&self, delta: Duration, transform: &mut Transform) {
        let new_scale = transform.scale * 1. + (self.speed * delta.as_secs_f32());
        transform.scale = Vec3::new(
            f32::min(f32::max(new_scale.x, 0.1), 20.),
            f32::min(f32::max(new_scale.y, 0.1), 20.),
            1.,
        );
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
            .add_event::<GameEvent>()
            .add_state::<GameState>()
            .add_systems(Startup, spawn_camera_system)
            .add_systems(
                OnEnter(GameState::Startup),
                (spawn_world, spawn_player_system, reset_camera_system),
            )
            .add_systems(
                Update,
                player_pause_system.run_if(in_state(GameState::Running)),
            )
            .add_systems(
                Update,
                player_unpause_system.run_if(in_state(GameState::Pause)),
            )
            .add_systems(
                Update,
                player_restart_system.run_if(in_state(GameState::Over)),
            )
            .add_systems(
                Update,
                (
                    player_move_system,
                    player_swap_scale_system,
                    apply_scale_system,
                    obstacle_factory_system,
                    spawn_obstacle_system,
                    despawn_out_of_view,
                    hit_obstacle_system,
                    bust_effect_system,
                    temporary_despawn_system,
                    time_score_system,
                    destroy_system,
                    enemy_system,
                )
                    .run_if(in_state(GameState::Running)),
            )
            .add_systems(Update, game_event_system);
    }
}

impl Default for GamePlugin {
    fn default() -> Self {
        Self {}
    }
}

pub fn reset_camera_system(mut query: Query<(&mut Transform), With<Camera>>) {
    for mut transform in query.iter_mut() {
        transform.translation = Vec3::ZERO;
    }
}

pub fn spawn_world(
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
    mut state: ResMut<NextState<GameState>>,
    mut time: ResMut<Time<Virtual>>,
) {
    time.unpause();
    state.set(GameState::Running);
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
    commands
        .spawn(
            TextBundle::from_section(
                "??:??",
                TextStyle {
                    font_size: 64.,
                    ..default()
                },
            )
            .with_text_alignment(TextAlignment::Center)
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(0.),
                right: Val::Percent(1.),
                ..default()
            }),
        )
        .insert(TimeScore::default());
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
            camera_transform.translation.truncate() + (camera_direction * 1080. / 2. + 64.);
        for _ in 0..2 {
            let scale = 0.75 + random.f32() * 0.50;
            let position =
                obstacle_middle + obstacle_direction * random.f32_normalized() * 720. / 2.;
            let kind = match random.u8(0..=4) {
                0 => ObstacleKind::ScaleBust(true),
                1 => ObstacleKind::ScaleBust(false),
                2 => ObstacleKind::Block,
                3 => ObstacleKind::Ice,
                4 => ObstacleKind::Poison,
                _ => ObstacleKind::Block,
            };
            event.send(SpawnObstacleEvent {
                color: kind.get_color(),
                position: position.extend(0.),
                radius: ORIGINAL_RADIUS,
                scale,
                kind,
            })
        }
    }
}

#[derive(Event, Debug)]
pub struct SpawnObstacleEvent {
    pub color: Color,
    pub position: Vec3,
    pub radius: f32,
    pub scale: f32,
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
            //.insert(CollidingEntities::default())
            //.insert(Sensor::default())
            .insert(ActiveEvents::all())
            //.insert(ActiveHooks::all())
            .insert(MaterialMesh2dBundle {
                mesh: circle.into(),
                material: material,
                transform: Transform::from_translation(event.position).with_scale(Vec3::new(
                    event.scale,
                    event.scale,
                    1.,
                )),
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
        .insert(CollisionGroups::new(Group::all(), Group::all()))
        .insert(SolverGroups::new(
            Group::from_bits_retain(0b1),
            Group::from_bits_retain(0b1),
        ))
        .insert(MaterialMesh2dBundle {
            mesh: circle.into(),
            material: material,
            transform: Transform::from_translation(Vec3::new(0., 0., 1.)),
            ..Default::default()
        });
}

pub fn player_move_system(
    mut commands: Commands,
    query: Query<(Entity, &ActionState<PlayerAction>, &Transform), With<Player>>,
    frozen_query: Query<&FrozenEffect>,
) {
    for (entity, action_state, transform) in query.iter() {
        if frozen_query.iter().any(|it| it.target == entity) {
            commands.entity(entity).insert(Velocity::zero());
            continue;
        }
        let speed = calc_speed(transform);
        if let Some(move_axis_pair) = action_state.axis_pair(PlayerAction::Move) {
            let direction = move_axis_pair.xy();
            let speed = direction.normalize_or_zero() * speed;
            commands.entity(entity).insert(Velocity::linear(speed));
        }
    }
}

pub fn player_pause_system(
    mut time: ResMut<Time<Virtual>>,
    mut state: ResMut<NextState<GameState>>,
    query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
) {
    for (_, action_state) in query.iter() {
        if action_state.just_pressed(PlayerAction::Pause) {
            if !time.is_paused() {
                state.set(GameState::Pause);
                time.pause();
            }
        }
    }
}

pub fn player_unpause_system(
    mut time: ResMut<Time<Virtual>>,
    mut state: ResMut<NextState<GameState>>,
    query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
) {
    for (_, action_state) in query.iter() {
        if action_state.just_pressed(PlayerAction::Pause) {
            if time.is_paused() {
                state.set(GameState::Running);
                time.unpause();
            }
        }
    }
}

pub fn player_restart_system(
    mut commands: Commands,
    query: Query<(Entity, &ActionState<PlayerAction>), With<Player>>,
    mut events: EventWriter<GameEvent>,
    clean_query: Query<(Entity), (Without<Camera>, Without<Window>)>,
) {
    for (_, action_state) in query.iter() {
        if action_state.just_released(PlayerAction::Start) {
            events.send(GameEvent::Start);
            for entity in clean_query.iter() {
                commands.entity(entity).despawn_recursive();
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

pub fn apply_scale_system(
    time: Res<Time>,
    mut query: Query<(Entity, &Scale, &mut Transform)>,
    frozen_query: Query<&FrozenEffect>,
) {
    for (entity, scale, mut transform) in query.iter_mut() {
        if frozen_query.iter().any(|it| it.target == entity) {
            continue;
        }
        scale.apply(time.delta(), &mut transform);
    }
}

pub fn despawn_out_of_view(
    mut commands: Commands,
    camera_query: Query<(&Transform, &Velocity), With<Camera>>,
    is_player: Query<Entity, With<Player>>,
    query: Query<(Entity, &ViewVisibility, &Transform)>,
    mut events: EventWriter<GameEvent>,
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
            .angle_between(camera_dir)
            .abs();
        if angle >= (90_f32).to_radians() && angle <= (270_f32).to_radians() {
            if let Ok(_) = is_player.get(entity) {
                events.send(GameEvent::GameOver);
            } else {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

pub fn hit_obstacle_system(
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mut player_query: Query<(Entity, &CollidingEntities, &Scale, &Transform), With<Player>>,
    obstacle_query: Query<(Entity, &Obstacle, &Transform)>,
    mut events: EventWriter<GameEvent>,
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
                        events.send(GameEvent::GameOver);
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

pub fn time_score_system(
    mut commands: Commands,
    time: Res<Time>,
    mut score_query: Query<(Entity, &mut TimeScore)>,
) {
    for (entity, mut score) in score_query.iter_mut() {
        score.tick(time.delta());
        commands.entity(entity).insert(Text::from_section(
            score.to_string(),
            TextStyle {
                font_size: 64.,
                ..default()
            },
        ));
    }
}

pub fn game_event_system(
    mut commands: Commands,
    mut time: ResMut<Time<Virtual>>,
    mut events: EventReader<GameEvent>,
    mut state: ResMut<NextState<GameState>>,
) {
    for event in events.read() {
        match event {
            GameEvent::GameOver => {
                time.pause();
                state.set(GameState::Over);
                commands.spawn(
                    TextBundle::from_section(
                        "Game Over",
                        TextStyle {
                            font_size: 64.,
                            ..default()
                        },
                    )
                    .with_style(Style {
                        align_content: AlignContent::Center,
                        top: Val::Auto,
                        left: Val::Auto,
                        width: Val::Percent(1.),
                        ..default()
                    }),
                );
            }
            GameEvent::Start => {
                state.set(GameState::Startup);
            }
        }
    }
}

pub fn destroy_system(
    mut commands: Commands,
    query: Query<(Entity, &Destroy)>,
    is_player: Query<(Entity), With<Player>>,
    mut events: EventWriter<GameEvent>,
) {
    for (destroy_entity, destroy) in query.iter() {
        let target = destroy.target;
        if let Ok(player) = is_player.get(target) {
            events.send(GameEvent::GameOver);
        } else {
            commands.entity(target).despawn();
        }
        commands.entity(destroy_entity).despawn();
    }
}

pub fn enemy_system(
    mut commands: Commands,
    enemy_query: Query<(Entity, &Enemy, &Transform)>,
    player_query: Query<(Entity, &Transform), With<Player>>,
) {
    let (player, player_transform) = player_query.get_single().unwrap();
    for (enemy, enemy_strategy, enemy_transform) in enemy_query.iter() {
        let velocity = enemy_strategy.tick(
            (enemy_transform, &Velocity::zero()),
            (player_transform, &Velocity::zero()),
        );
        commands.entity(enemy).try_insert(velocity);
    }
}
