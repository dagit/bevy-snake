use bevy::app::AppExit;
use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    prelude::*,
};

use std::collections::VecDeque;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<FoodCollisionEvent>()
        .add_event::<GameOverEvent>()
        .insert_resource(MoveTimer(Timer::from_seconds(0.3, TimerMode::Repeating)))
        .insert_resource(FoodSpawnTimer(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .init_state::<GameState>()
        .add_systems(Startup, (setup_camera, load_audio))
        // Main menu
        .add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(Update, menu.run_if(in_state(GameState::Menu)))
        .add_systems(OnExit(GameState::Menu), cleanup_menu)
        // Clean slate
        .add_systems(
            OnEnter(GameState::StartGame),
            (cleanup_system::<CleanupOnRestart>, add_snake).chain(),
        )
        // Main game play loop
        .add_systems(
            Update,
            (
                input_direction,
                input_pause,
                move_snake,
                spawn_food,
                animate_food,
                wall_collision_check,
                self_collision_check,
                food_collision_check,
                game_over_check,
                grow,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        )
        // Paused
        .add_systems(OnEnter(GameState::Pause), setup_pause)
        .add_systems(Update, paused.run_if(in_state(GameState::Pause)))
        .add_systems(OnExit(GameState::Pause), cleanup_pause)
        // Game Over
        .add_systems(OnEnter(GameState::GameOver), setup_game_over)
        .add_systems(
            Update,
            (game_over_retry_button, game_over_quit_button).run_if(in_state(GameState::GameOver)),
        )
        .add_systems(OnExit(GameState::GameOver), cleanup_game_over)
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Menu,
    StartGame,
    InGame,
    Pause,
    GameOver,
}

#[derive(Resource)]
struct MenuData {
    button: Entity,
}

#[derive(Resource)]
struct PauseData {
    button: Entity,
}

#[derive(Resource)]
struct GameOverData {
    buttons: Entity,
}

#[derive(Event)]
struct FoodCollisionEvent;

#[derive(Event)]
struct GameOverEvent;

#[derive(Component)]
struct Length(i32);

#[derive(Component, Eq, PartialEq)]
enum Direction {
    North,
    East,
    West,
    South,
}

#[derive(Component, Default)]
struct PlayerControlled;

impl Direction {
    fn to_x(&self) -> f32 {
        match self {
            Direction::North => 0.,
            Direction::East => 1.,
            Direction::West => -1.,
            Direction::South => 0.,
        }
    }
    fn to_y(&self) -> f32 {
        match self {
            Direction::North => 1.,
            Direction::East => 0.,
            Direction::West => 0.,
            Direction::South => -1.,
        }
    }
}

#[derive(Component)]
struct Food;

#[derive(Resource)]
struct MoveTimer(Timer);

#[derive(Resource)]
struct FoodSpawnTimer(Timer);

#[derive(Bundle)]
struct Segment {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
}

const SEGMENT_SIZE: f32 = 10.0;
const SNAKE_COLOR: Srgba = Srgba::new(1.0, 0.0, 0.0, 1.0);
const FOOD_COLOR: Srgba = Srgba::new(0.1, 1.0, 0.0, 1.0);

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

impl Segment {
    fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        x: f32,
        y: f32,
    ) -> Self {
        Segment {
            transform: Transform::from_xyz(x, y, 0.0),
            mesh: Mesh2d(meshes.add(Rectangle::new(SEGMENT_SIZE, SEGMENT_SIZE))),
            material: MeshMaterial2d(materials.add(ColorMaterial::from_color(SNAKE_COLOR))),
        }
    }
}

#[derive(Component)]
struct Segments(VecDeque<Entity>);

#[derive(Bundle)]
struct SnakeBundle {
    desired_len: Length,
    segments: Segments,
    dir: Direction,
    player: PlayerControlled,
}

#[derive(Component)]
struct CleanupOnRestart;

fn cleanup_system<T: Component>(mut commands: Commands, q: Query<Entity, With<T>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}

impl SnakeBundle {
    fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<ColorMaterial>>,
        commands: &mut Commands,
    ) -> Self {
        let segment = commands
            .spawn((
                Name::new("segment"),
                CleanupOnRestart,
                Segment::new(meshes, materials, 0., 0.),
            ))
            .id();
        let mut vec = VecDeque::new();
        vec.push_back(segment);
        SnakeBundle {
            desired_len: Length(10),
            segments: Segments(vec),
            dir: Direction::North,
            player: PlayerControlled,
        }
    }
}

fn setup_menu(mut commands: Commands) {
    let button = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Start"),
                        TextFont {
                            font_size: 33.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        })
        .id();
    commands.insert_resource(MenuData { button });
}

fn menu(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    keys: Res<ButtonInput<KeyCode>>,
    menu_sound: Res<MenuRolloverSound>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                next_state.set(GameState::StartGame);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                commands.spawn(AudioPlayer(menu_sound.0.clone()));
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::StartGame);
    }
}

fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>) {
    commands.entity(menu_data.button).despawn_recursive();
}

#[derive(Component)]
pub struct RetryButton;
#[derive(Component)]
pub struct QuitButton;

fn setup_game_over(mut commands: Commands) {
    let buttons = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    RetryButton,
                    Button,
                    Node {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Retry"),
                        TextFont {
                            font_size: 33.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
            parent
                .spawn((
                    QuitButton,
                    Button,
                    Node {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Quit"),
                        TextFont {
                            font_size: 33.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        })
        .id();
    commands.insert_resource(GameOverData { buttons });
}

fn game_over_retry_button(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<RetryButton>),
    >,
    keys: Res<ButtonInput<KeyCode>>,
    menu_sound: Res<MenuRolloverSound>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                next_state.set(GameState::StartGame);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                commands.spawn(AudioPlayer(menu_sound.0.clone()));
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::StartGame);
    }
}

fn game_over_quit_button(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<QuitButton>),
    >,
    menu_sound: Res<MenuRolloverSound>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                exit.send(AppExit::Success);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                commands.spawn(AudioPlayer(menu_sound.0.clone()));
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn cleanup_game_over(mut commands: Commands, game_over_data: Res<GameOverData>) {
    commands.entity(game_over_data.buttons).despawn_recursive();
}

fn setup_pause(mut commands: Commands) {
    let button = commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Unpause"),
                        TextFont {
                            font_size: 33.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        })
        .id();
    commands.insert_resource(PauseData { button });
}

fn paused(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    keys: Res<ButtonInput<KeyCode>>,
    menu_sound: Res<MenuRolloverSound>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                next_state.set(GameState::InGame);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                commands.spawn(AudioPlayer(menu_sound.0.clone()));
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
    if keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Escape)
    {
        next_state.set(GameState::InGame);
    }
}

fn cleanup_pause(mut commands: Commands, pause_data: Res<PauseData>) {
    commands.entity(pause_data.button).despawn_recursive();
}

fn add_snake(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    start_sound: Res<StartSound>,
) {
    let snake = SnakeBundle::new(&mut meshes, &mut materials, &mut commands);
    commands.spawn((Name::new("snake"), CleanupOnRestart, snake));
    commands.spawn(AudioPlayer(start_sound.0.clone()));
    next_state.set(GameState::InGame);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Bloom::default(),
    ));
}

fn move_snake(
    time: Res<Time>,
    mut timer: ResMut<MoveTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(&mut Segments, &Length, &Direction)>,
    mut segment_query: Query<&mut Transform>,
    mut commands: Commands,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for (mut segments, len, dir) in &mut query {
            if len.0 as usize <= segments.0.len() {
                let head = len.0.saturating_sub(1) as usize;
                let head_segment = *segment_query.get(segments.0[head]).unwrap();
                let tail = segments.0.pop_front().unwrap();
                if let Ok(mut tail_transform) = segment_query.get_mut(tail) {
                    tail_transform.translation.x =
                        head_segment.translation.x + dir.to_x() * SEGMENT_SIZE;
                    tail_transform.translation.y =
                        head_segment.translation.y + dir.to_y() * SEGMENT_SIZE;
                    segments.0.push_back(tail);
                }
            } else if let Ok(head_segment) = segment_query.get_mut(segments.0[segments.0.len() - 1])
            {
                let new_x = head_segment.translation.x + dir.to_x() * SEGMENT_SIZE;
                let new_y = head_segment.translation.y + dir.to_y() * SEGMENT_SIZE;
                let segment = commands
                    .spawn((
                        Name::new("segment"),
                        CleanupOnRestart,
                        Segment::new(&mut meshes, &mut materials, new_x, new_y),
                    ))
                    .id();
                segments.0.push_back(segment);
            }
        }
    }
}

fn input_direction(
    keys: Res<ButtonInput<KeyCode>>,
    mut direction: Query<&mut Direction, With<PlayerControlled>>,
) {
    for mut dir in &mut direction {
        if keys.just_pressed(KeyCode::ArrowLeft) && *dir != Direction::East {
            *dir = Direction::West;
        } else if keys.just_pressed(KeyCode::ArrowRight) && *dir != Direction::West {
            *dir = Direction::East;
        } else if keys.just_pressed(KeyCode::ArrowUp) && *dir != Direction::South {
            *dir = Direction::North;
        } else if keys.just_pressed(KeyCode::ArrowDown) && *dir != Direction::North {
            *dir = Direction::South;
        }
    }
}

fn input_pause(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Pause);
    }
}

fn spawn_food(
    time: Res<Time>,
    mut timer: ResMut<FoodSpawnTimer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    segment_transform: Query<&Transform>,
    windows: Query<&mut Window>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let window = windows.single();
        let width = window.resolution.width();
        let height = window.resolution.height();
        let x_uniform = rand::distributions::Uniform::new_inclusive(
            -width / SEGMENT_SIZE / 2.,
            width / SEGMENT_SIZE / 2.,
        );
        let y_uniform = rand::distributions::Uniform::new_inclusive(
            -height / SEGMENT_SIZE / 2.,
            height / SEGMENT_SIZE / 2.,
        );
        let x = (rng.sample(x_uniform).round() * SEGMENT_SIZE).round();
        let y = (rng.sample(y_uniform).round() * SEGMENT_SIZE).round();
        for segment in &segment_transform {
            // Don't place the food on top of the snek
            if segment.translation.x == x && segment.translation.y == y {
                return;
            }
        }
        let food = Name::new("food");

        let mut animation = AnimationClip::default();
        let food_animation_target_id = AnimationTargetId::from_name(&food);
        animation.add_curve_to_target(
            food_animation_target_id,
            AnimatableCurve::new(
                animated_field!(Transform::scale),
                UnevenSampleAutoCurve::new([0.0, 1.0, 2.0].into_iter().zip([
                    Vec3::splat(0.5),
                    Vec3::splat(1.0),
                    Vec3::splat(0.5),
                ]))
                .unwrap(),
            ),
        );
        let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));
        let mut animation_player = AnimationPlayer::default();
        animation_player.play(animation_index).repeat();

        let food_id = commands
            .spawn((
                food,
                Food,
                CleanupOnRestart,
                Transform::from_xyz(x, y, 0.0),
                Mesh2d(meshes.add(Rectangle::new(SEGMENT_SIZE, SEGMENT_SIZE))),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(FOOD_COLOR))),
                AnimationGraphHandle(graphs.add(graph)),
                animation_player,
            ))
            .id();
        commands.entity(food_id).insert(AnimationTarget {
            id: food_animation_target_id,
            player: food_id,
        });
    }
}

fn animate_food(
    material_handles: Query<&MeshMaterial2d<ColorMaterial>, With<Food>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for material_handle in material_handles.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            let hsla: Hsla = material.color.into();
            *material = ColorMaterial::from_color(hsla.rotate_hue(time.delta_secs() * 100.0));
        }
    }
}

fn food_collision_check(
    mut commands: Commands,
    mut food_collision_writer: EventWriter<FoodCollisionEvent>,
    food: Query<(Entity, &Transform), With<Food>>,
    segment_transform: Query<&Transform>,
    segments: Query<(&Segments, &Length), With<PlayerControlled>>,
) {
    for (segments, len) in &segments {
        let head_idx = if len.0 as usize <= segments.0.len() {
            len.0.saturating_sub(1) as usize
        } else {
            segments.0.len() - 1
        };
        let head_transform = *segment_transform.get(segments.0[head_idx]).unwrap();
        for (id, transform) in food.iter() {
            if transform.translation.x == head_transform.translation.x
                && transform.translation.y == head_transform.translation.y
            {
                commands.entity(id).despawn();
                food_collision_writer.send(FoodCollisionEvent);
            }
        }
    }
}

fn grow(
    mut commands: Commands,
    eat_sound: Res<EatSound>,
    mut food_collision_reader: EventReader<FoodCollisionEvent>,
    mut snake: Query<&mut Length, With<PlayerControlled>>,
) {
    if food_collision_reader.read().next().is_some() {
        commands.spawn(AudioPlayer(eat_sound.0.clone()));
        for mut len in &mut snake {
            let Length(l) = *len;
            *len = Length(l + 10);
        }
    }
}

fn wall_collision_check(
    mut game_over_writer: EventWriter<GameOverEvent>,
    segment_transform: Query<&Transform>,
    segments: Query<(&Segments, &Length), With<PlayerControlled>>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let width = window.resolution.width();
    let height = window.resolution.height();
    for (segments, len) in &segments {
        // TODO: can I just make this peek the back?
        let head_idx = if len.0 as usize <= segments.0.len() {
            len.0.saturating_sub(1) as usize
        } else {
            segments.0.len() - 1
        };
        let head_transform = *segment_transform.get(segments.0[head_idx]).unwrap();
        if head_transform.translation.x > width / 2.
            || head_transform.translation.x < -width / 2.
            || head_transform.translation.y > height / 2.
            || head_transform.translation.y < -height / 2.
        {
            game_over_writer.send(GameOverEvent);
        }
    }
}

fn self_collision_check(
    mut game_over_writer: EventWriter<GameOverEvent>,
    segment_transform: Query<&Transform>,
    segments: Query<(&Segments, &Length), With<PlayerControlled>>,
) {
    for (segments, len) in &segments {
        // TODO: can I just make this peek the back?
        let head_idx = if len.0 as usize <= segments.0.len() {
            len.0.saturating_sub(1) as usize
        } else {
            segments.0.len() - 1
        };
        let head_transform = *segment_transform.get(segments.0[head_idx]).unwrap();
        for (idx, segment) in segments.0.iter().enumerate() {
            if idx == head_idx {
                continue;
            }
            let s = segment_transform.get(*segment).unwrap();
            if s.translation.x == head_transform.translation.x
                && s.translation.y == head_transform.translation.y
            {
                game_over_writer.send(GameOverEvent);
            }
        }
    }
}

fn game_over_check(
    mut commands: Commands,
    mut game_over_reader: EventReader<GameOverEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    crash_sound: Res<CrashSound>,
) {
    if game_over_reader.read().next().is_some() {
        next_state.set(GameState::GameOver);
        commands.spawn(AudioPlayer(crash_sound.0.clone()));
    }
}

#[derive(Resource)]
pub struct EatSound(Handle<AudioSource>);
#[derive(Resource)]
pub struct MenuRolloverSound(Handle<AudioSource>);
#[derive(Resource)]
pub struct CrashSound(Handle<AudioSource>);
#[derive(Resource)]
pub struct StartSound(Handle<AudioSource>);

fn load_audio(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<AudioSource> = server.load("eat.wav");
    commands.insert_resource(EatSound(handle));
    let handle: Handle<AudioSource> = server.load("menu-rollover.wav");
    commands.insert_resource(MenuRolloverSound(handle));
    let handle: Handle<AudioSource> = server.load("crash.wav");
    commands.insert_resource(CrashSound(handle));
    let handle: Handle<AudioSource> = server.load("start.wav");
    commands.insert_resource(StartSound(handle));
}
