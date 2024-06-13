use bevy::{asset::AssetMetaCheck, prelude::*, render::render_asset::RenderAssetUsages};
use bevy_async_task::{AsyncTaskPool, AsyncTaskStatus};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AssetMetaCheck::Never)
        .insert_resource(TaskQueue(vec![]))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy GIS".to_string(),
                    canvas: Some("#bevy".to_owned()),
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
            WorldInspectorPlugin::new(),
            PanCamPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (tile_system, sort_by_distance))
        .run();
}

#[derive(Debug, Clone)]
struct TilePos {
    zoom: i32,
    y: i32,
    x: i32,
}

struct Tile {
    tile_pos: TilePos,
    image: Image,
}

#[derive(Resource)]
pub struct TaskQueue(Vec<TilePos>);

fn setup(mut commands: Commands, mut task_queue: ResMut<TaskQueue>) {
    commands.spawn(Camera2dBundle::default()).insert(PanCam {
        min_x: Some(-1000.),
        max_x: Some(1000.),
        min_y: Some(-1000.),
        max_y: Some(1000.),
        ..default()
    });

    let zoom = 6;
    let pow = 2i32.pow(zoom as u32);

    for x in 0..pow {
        for y in 0..pow {
            task_queue.0.push(TilePos { zoom, y, x });
        }
    }
}

fn sort_by_distance(
    mut task_queue: ResMut<TaskQueue>,
) {
    task_queue.0.sort_unstable_by(|a, b| {
        convert_pos(b)
            .distance(Vec2::ZERO)
            .partial_cmp(&convert_pos(a).distance(Vec2::ZERO))
            .unwrap()
    });
}

fn convert_pos(tile_pos: &TilePos) -> Vec2 {
    let TilePos { zoom, y, x } = tile_pos.to_owned();

    let map_size = 2000.;
    let sprite_size = map_size / 2f32.powf(zoom as f32);
    let offset = (sprite_size - map_size) / 2.;

    Vec2::new(
        x as f32 * sprite_size + offset,
        -y as f32 * sprite_size - offset,
    )
}

fn tile_system(
    mut task_pool: AsyncTaskPool<Tile>,
    mut task_queue: ResMut<TaskQueue>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let pending = task_pool.iter_poll().fold(0, |acc, status| match status {
        AsyncTaskStatus::Pending => acc + 1,
        AsyncTaskStatus::Finished(tile) => {
            let texture_handle = asset_server.add(tile.image.clone());

            let Vec2 { x, y } = convert_pos(&tile.tile_pos);

            let map_size = 2000.;
            let sprite_size = map_size / 2f32.powf(tile.tile_pos.zoom as f32);

            commands.spawn(SpriteBundle {
                texture: texture_handle,
                transform: Transform::from_xyz(x, y, 0.),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(sprite_size)),
                    ..default()
                },
                ..default()
            });

            acc
        }
        _ => acc,
    });

    let max_concurrent_tasks = 10;
    let remaining_tasks = task_queue.0.len();

    if pending >= max_concurrent_tasks || remaining_tasks == 0 {
        return;
    };

    let index = remaining_tasks - (max_concurrent_tasks - pending).min(remaining_tasks);

    for tile_pos in task_queue.0.split_off(index) {
        task_pool.spawn(async move {

                let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
                let server_url = "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";

                let TilePos { zoom, y, x } = tile_pos;

                let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

                let bytes = reqwest::get(&full_url)
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();

                let dyn_image = image::load_from_memory(&bytes).expect("cant load image");

                let image = Image::from_dynamic(
                    dyn_image,
                    true,
                    RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
                );

            Tile { tile_pos, image }
        });
    }
}
