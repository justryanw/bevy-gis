use std::collections::HashMap;

use bevy::{asset::AssetMetaCheck, prelude::*, render::render_asset::RenderAssetUsages};
use bevy_async_task::{AsyncTaskPool, AsyncTaskStatus};
use bevy_pancam::{PanCam, PanCamPlugin};

#[derive(Debug, Clone, Component, PartialEq, Eq, Hash)]
struct TilePos {
    zoom: i32,
    y: i32,
    x: i32,
}

struct Tile {
    tile_pos: TilePos,
    image: Image,
}

#[derive(Debug, PartialEq, Eq)]
enum TileStatus {
    Queued,
    Pending,
    Complete(Entity),
}

#[derive(Resource, Debug)]
pub struct Tiles(HashMap<TilePos, TileStatus>);

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AssetMetaCheck::Never)
        .insert_resource(Tiles(HashMap::new()))
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
            // WorldInspectorPlugin::new(),
            PanCamPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_system, tile_system).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default()).insert(PanCam {
        min_x: Some(-1000.),
        max_x: Some(1000.),
        min_y: Some(-1000.),
        max_y: Some(1000.),
        ..default()
    });
}

fn spawn_system(
    query: Query<(&OrthographicProjection, &Transform), With<PanCam>>,
    mut tiles: ResMut<Tiles>,
) {
    let (otho_projection, transform) = query.single();
    let (min, max) = swap_rect_y(otho_projection.area);
    let cam_pos = transform.translation.xy();

    let ratio = 2000. / otho_projection.area.height();
    let max_zoom = ((4. + ratio.log2()) as i32).clamp(1, 20);

    // info!("height {} {} {}", height, ch, otho_projection.area.height());

    for zoom in 1..max_zoom {
        let min_tile = wolrd_to_tile_pos(min + cam_pos, zoom).floor();
        let max_tile = wolrd_to_tile_pos(max + cam_pos, zoom).ceil();

        for x in (min_tile.x as i32)..(max_tile.x as i32) {
            for y in (min_tile.y as i32)..(max_tile.y as i32) {
                let tile_pos = TilePos { zoom, y, x };
                if tiles.0.contains_key(&tile_pos) {
                    continue;
                };
                tiles.0.insert(tile_pos.clone(), TileStatus::Queued);
            }
        }
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
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut tiles: ResMut<Tiles>,
    query: Query<&Transform, With<PanCam>>,
) {
    let transform = query.single();
    let cam_pos = Vec2::new(transform.translation.x, transform.translation.y);

    task_pool.iter_poll().for_each(|status| match status {
        AsyncTaskStatus::Finished(tile) => {
            let texture_handle = asset_server.add(tile.image);

            let (Vec2 { x, y }, sprite_size) = convert_pos(&tile.tile_pos);

            let entity = commands
                .spawn((
                    SpriteBundle {
                        texture: texture_handle,
                        transform: Transform::from_xyz(x, y, tile.tile_pos.zoom as f32),
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(sprite_size)),
                            ..default()
                        },
                        ..default()
                    },
                    tile.tile_pos.clone(),
                ))
                .id();

            tiles
                .0
                .insert(tile.tile_pos.clone(), TileStatus::Complete(entity));
        }
        _ => (),
    });

    let mut queued = 0;
    let mut pending = 0;
    let mut task_queue: Vec<TilePos> = vec![];

    tiles.0.iter().for_each(|(k, v)| match v {
        TileStatus::Queued => {
            queued += 1;
            task_queue.push(k.clone());
        }
        TileStatus::Pending => {
            pending += 1;
        }
        _ => (),
    });

    task_queue.sort_by(|a, b| {
        a.zoom.cmp(&b.zoom).then_with(|| {
            let dist_b = convert_pos(b).0.distance(cam_pos);
            let dist_a = convert_pos(a).0.distance(cam_pos);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
    });

    let max_concurrent_tasks = 10;

    if pending >= max_concurrent_tasks || queued == 0 {
        return;
    };

    let index = (max_concurrent_tasks - pending).min(queued);

    for tile_pos in task_queue.split_at(index).0 {
        let tile_pos = tile_pos.clone();
        let TilePos { zoom, y, x } = tile_pos;

        tiles.0.insert(tile_pos.clone(), TileStatus::Pending);

        task_pool.spawn(async move {
            // Ersi
            let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
            let server_url = "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";
            let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

            // OSM
            // let server_url = "https://tile.openstreetmap.org";
            // let full_url = format!("{server_url}/{zoom}/{x}/{y}.png");

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
