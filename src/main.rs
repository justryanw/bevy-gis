use bevy::{asset::AssetMetaCheck, prelude::*, render::render_asset::RenderAssetUsages};
use bevy_async_task::{AsyncTaskPool, AsyncTaskStatus};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};

#[derive(Debug, Clone, Component, PartialEq)]
struct TilePos {
    zoom: i32,
    y: i32,
    x: i32,
}

struct Tile {
    tile_pos: TilePos,
    image: Image,
}

#[derive(Resource, Debug)]
pub struct TaskQueue(Vec<TilePos>);

#[derive(Resource)]
pub struct PendingTasks(Vec<TilePos>);

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AssetMetaCheck::Never)
        .insert_resource(TaskQueue(vec![]))
        .insert_resource(PendingTasks(vec![]))
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
        .add_systems(
            Update,
            (spawn_system, sort_by_distance, tile_system).chain(),
        )
        .run();
}

fn setup(mut commands: Commands, mut task_queue: ResMut<TaskQueue>) {
    commands.spawn(Camera2dBundle::default()).insert(PanCam {
        min_x: Some(-1000.),
        max_x: Some(1000.),
        min_y: Some(-1000.),
        max_y: Some(1000.),
        ..default()
    });

    let zoom = 4;
    let pow = 2i32.pow(zoom as u32);

    // TODO replace with spawn system
    // for x in 0..pow {
    //     for y in 0..pow {
    //         task_queue.0.push(TilePos { zoom, y, x });
    //     }
    // }
}

fn spawn_system(
    query: Query<(&OrthographicProjection, &Transform), With<PanCam>>,
    mut gizmos: Gizmos,
    mut task_queue: ResMut<TaskQueue>,
    pending_tasks: Res<PendingTasks>,
    tile_query: Query<&TilePos>,
) {
    let (otho_projection, transform) = query.single();
    let (min, max) = swap_rect_y(otho_projection.area);
    let cam_pos = transform.translation.xy();

    // gizmos.circle_2d(min + cam_pos, 100., Color::RED);
    // gizmos.circle_2d(max + cam_pos, 100., Color::BLUE);

    task_queue.0 = vec![];

    let height = ((50. / (min.y - max.y).log2()) as i32).clamp(1, 16);

    info!("height {}", height);

    for zoom in 1..height {
        let min_tile = wolrd_to_tile_pos(min + cam_pos, zoom).floor();
        let max_tile = wolrd_to_tile_pos(max + cam_pos, zoom).ceil();
        // info!("min {} max {}", min_tile, max_tile);

        let tile_min = tile_to_world_pos(min_tile, zoom);
        let tile_max = tile_to_world_pos(max_tile, zoom);
        // info!("min {} max {}", tile_min, tile_max);

        let snapped_rect = Rect::from_corners(tile_min, tile_max);

        // gizmos.rect_2d(snapped_rect.center(), 0., snapped_rect.size(), Color::GREEN);

        for x in (min_tile.x as i32)..(max_tile.x as i32) {
            for y in (min_tile.y as i32)..(max_tile.y as i32) {
                let tile_pos = TilePos { zoom, y, x };
                if pending_tasks.0.contains(&tile_pos) {
                    continue;
                };
                // if task_queue.0.contains(&tile_pos) {
                //     continue;
                // }
                if tile_query.iter().find(|&t| *t == tile_pos).is_some() {
                    continue;
                }
                // info!("tilepos {:?}", tile_pos);
                task_queue.0.push(tile_pos.clone());
            }
        }

        // info!("cam pos {}", cam_pos);
    }
}

fn swap_rect_y(rect: Rect) -> (Vec2, Vec2) {
    let Rect { min, max } = rect;
    (Vec2::new(min.x, max.y), Vec2::new(max.x, min.y))
}

fn wolrd_to_tile_pos(wold_pos: Vec2, zoom: i32) -> Vec2 {
    let map_size = 2000.;
    let pow = 2f32.powf(zoom as f32);
    let sprite_size = map_size / pow;

    Vec2::new(
        wold_pos.x / sprite_size + pow / 2.,
        -wold_pos.y / sprite_size + pow / 2.,
    )
    .clamp(Vec2::ZERO, Vec2::splat(pow))
}

fn tile_to_world_pos(tile_pos: Vec2, zoom: i32) -> Vec2 {
    let map_size = 2000.;
    let pow = 2f32.powf(zoom as f32);
    let sprite_size = map_size / pow;
    Vec2::new(
        (tile_pos.x - pow / 2.) * sprite_size,
        -(tile_pos.y - pow / 2.) * sprite_size,
    )
}

fn sort_by_distance(mut task_queue: ResMut<TaskQueue>, query: Query<&Transform, With<PanCam>>) {
    let transform = query.single();

    let cam_pos = Vec2::new(transform.translation.x, transform.translation.y);

    task_queue.0.sort_by(|a, b| {
        a.zoom.cmp(&b.zoom).then_with(|| {
            let dist_b = convert_pos(b).0.distance(cam_pos);
            let dist_a = convert_pos(a).0.distance(cam_pos);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
    });

    // info!("{:?}", task_queue);
}

fn convert_pos(tile_pos: &TilePos) -> (Vec2, f32) {
    let TilePos { zoom, y, x } = tile_pos.to_owned();

    let map_size = 2000.;
    let sprite_size = map_size / 2f32.powf(zoom as f32);
    let offset = (sprite_size - map_size) / 2.;

    (
        Vec2::new(
            x as f32 * sprite_size + offset,
            -y as f32 * sprite_size - offset,
        ),
        sprite_size,
    )
}

fn tile_system(
    mut task_pool: AsyncTaskPool<Tile>,
    mut task_queue: ResMut<TaskQueue>,
    mut pending_tasks: ResMut<PendingTasks>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let pending = task_pool.iter_poll().fold(0, |acc, status| match status {
        AsyncTaskStatus::Pending => acc + 1,
        AsyncTaskStatus::Finished(tile) => {
            let texture_handle = asset_server.add(tile.image);

            let (Vec2 { x, y }, sprite_size) = convert_pos(&tile.tile_pos);

            pending_tasks.0.retain(|value| *value != tile.tile_pos);

            commands.spawn((
                SpriteBundle {
                    texture: texture_handle,
                    transform: Transform::from_xyz(x, y, tile.tile_pos.zoom as f32),
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(sprite_size)),
                        ..default()
                    },
                    ..default()
                },
                tile.tile_pos,
            ));

            acc
        }
        _ => acc,
    });

    let max_concurrent_tasks = 10;
    let remaining_tasks = task_queue.0.len();

    if pending >= max_concurrent_tasks || remaining_tasks == 0 {
        return;
    };

    let index = (max_concurrent_tasks - pending).min(remaining_tasks);

    for tile_pos in task_queue.0.split_at(index).0 {
        let tile_pos = tile_pos.clone();
        let TilePos { zoom, y, x } = tile_pos;
        pending_tasks.0.push(tile_pos.clone());

        task_pool.spawn(async move {
         	// Ersi
            let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
            let server_url = "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";
            let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

            // OSM
            // let server_url = "https://tile.openstreetmap.org";
            // let full_url = format!("{server_url}/{zoom}/{y}/{x}");

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
