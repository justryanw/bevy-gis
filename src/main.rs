use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        texture::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType},
    },
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bytes::Bytes;

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AssetMetaCheck::Never)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy GIS".to_string(),
                canvas: Some("#bevy".to_owned()),
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, spawn_tile))
        .run();
}

#[derive(Component)]
struct FetchTile(Task<Bytes>);

fn spawn_tile(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut fetch_tasks: Query<&mut FetchTile>,
) {
    for mut task in &mut fetch_tasks {
        if let Some(bytes) = block_on(future::poll_once(&mut task.0)) {
            let image = Image::from_buffer(
                &bytes,
                ImageType::Format(ImageFormat::Jpeg),
                CompressedImageFormats::NONE,
                false,
                ImageSampler::linear(),
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            )
            .expect("Failed to get image from bytes");

            let texture_handle = asset_server.add(image);

            commands.spawn(SpriteBundle {
                texture: texture_handle,
                ..default()
            });
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
    let server_url =
        "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";
    let zoom = 1;
    let x = 1;
    let y = 1;

    let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

    let thread_pool = AsyncComputeTaskPool::get();

    let entity = commands.spawn_empty().id();
    let task = thread_pool.spawn(async move {
        let response = reqwest::get(full_url).await.expect("Failed to fetch tile");
        response
            .bytes()
            .await
            .expect("Failed to read bytes from response")
    });

    commands.entity(entity).insert(FetchTile(task));

    // commands.spawn(SpriteBundle {
    //     texture: asset_server.load("bevy.png"),
    //     ..default()
    // });
}

fn rotate(mut query: Query<&mut Transform, With<Sprite>>, time: Res<Time>) {
    for mut bevy in &mut query {
        bevy.rotate_local_z(-time.delta_seconds());
    }
}
