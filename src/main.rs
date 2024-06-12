use std::time::Duration;

use async_std::task::sleep;
use bevy::{asset::AssetMetaCheck, prelude::*, utils::synccell::SyncCell};
use bevy_async_task::{AsyncTaskPool, AsyncTaskStatus};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
        .insert_resource(AsyncTaskPool(SyncCell::new(vec![])))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AssetMetaCheck::Never)
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
        .add_systems(Update, system1)
        .run();
}

struct ImageToDigest(Image);

struct TilePos {
    zoom: i32,
    y: i32,
    x: i32,
}

#[derive(Component)]
struct TileTask {
    tile_pos: TilePos,
    image: ImageToDigest,
}

// #[derive(Resource)]
// struct TileTaskPool<'a>(AsyncTaskPool<'a, TileTask>);

fn setup(
    mut commands: Commands,
    // mut task_pool: ResMut<TileTaskPool>, mut init_task_pool: AsyncTaskPool<TileTask>
) {
    commands.spawn(Camera2dBundle::default()).insert(PanCam {
        min_x: Some(-1000.),
        max_x: Some(1000.),
        min_y: Some(-1000.),
        max_y: Some(1000.),
        ..default()
    });

    // commands.insert_resource(TileTaskPool(init_task_pool));

    // let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
    // let server_url =
    //     "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";

    // let zoom = 2;
    // let pow = 2i32.pow(zoom as u32);

    // for x in 0..pow {
    //     for y in 0..pow {
    //         task_pool.0.spawn(async move {
    //             let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

    //             let bytes = reqwest::get(&full_url)
    //                 .await
    //                 .unwrap()
    //                 .bytes()
    //                 .await
    //                 .unwrap();

    //             let dyn_image = image::load_from_memory(&bytes).expect("cant load image");

    //             let image = Image::from_dynamic(
    //                 dyn_image,
    //                 true,
    //                 RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    //             );

    //             sleep(Duration::from_secs(2)).await;

    //             info!("TEST TEST TEST TEST {} {}", x, y);

    //             TileTask {
    //                 tile_pos: TilePos { zoom, y, x },
    //                 image: ImageToDigest(image),
    //             }
    //         });
    //     }
    // }
}

// fn digest_image(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     mut task_pool: ResMut<TileTaskPool>,
// ) {
//     for status in task_pool.0.iter_poll() {
//         info!("TEST 123123123123");
//         if let AsyncTaskStatus::Finished(tile_task) = status {
//             let texture_handle = asset_server.add(tile_task.image.0.clone());

//             info!("TEST 5555555555555");

//             let tile_pos = tile_task.tile_pos;

//             let map_size = 2000.;
//             let sprite_size = map_size / 2f32.powf(tile_pos.zoom as f32);
//             let offset = (sprite_size - map_size) / 2.;

//             commands.spawn(SpriteBundle {
//                 texture: texture_handle,
//                 transform: Transform::from_xyz(
//                     tile_pos.x as f32 * sprite_size + offset,
//                     -tile_pos.y as f32 * sprite_size - offset,
//                     0.,
//                 ),
//                 sprite: Sprite {
//                     custom_size: Some(Vec2::splat(sprite_size)),
//                     ..default()
//                 },
//                 ..default()
//             });
//         }
//     }
// }

// fn system1(mut task_pool: AsyncTaskPool<u64>) {
//     if task_pool.is_idle() {
//         println!("Queueing 5 tasks...");
//         for i in 1..=5 {
//             task_pool.spawn(async move {
//                 sleep(Duration::from_millis(i * 1000)).await;
//                 i
//             });
//         }
//     }

//     for status in task_pool.iter_poll() {
//         if let AsyncTaskStatus::Finished(t) = status {
//             info!("Received {t}");
//         }
//     }
// }

#[derive(Resource)]
pub struct MyAnswer(Option<u64>);

fn system1(mut task_pool: AsyncTaskPool<u64>, my_res: ResMut<MyAnswer>) {
    if task_pool.is_idle() {
        println!("Queueing 5 tasks...");
        for i in 1..=5 {
            task_pool.spawn(async move {
                sleep(Duration::from_millis(i * 1000)).await;
                i
            });
        }
    }

    

}

fn system2(mut task_pool: AsyncTaskPool<u64>) {
    for status in task_pool.iter_poll() {
        if let AsyncTaskStatus::Finished(t) = status {
            info!("Received {t}");
        }
    }
}
