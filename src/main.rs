use bevy::{asset::AssetMetaCheck, prelude::*, render::render_asset::RenderAssetUsages};
use bevy_mod_reqwest::{
    bevy_eventlistener::callbacks::ListenerInput, BevyReqwest, On, ReqResponse, ReqwestPlugin,
};

fn main() {
    console_log::init().expect("Error initialising logger");

    App::new()
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
            ReqwestPlugin::default(),
        ))
        .add_event::<FetchTile>()
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, spawn_tile))
        .run();
}

#[derive(Debug, Event)]
pub struct FetchTile(Image);

impl From<ListenerInput<ReqResponse>> for FetchTile {
    fn from(value: ListenerInput<ReqResponse>) -> Self {
        let bytes = value.body();

        let dyn_image = image::load_from_memory(bytes).expect("cant load image");

        let image = Image::from_dynamic(
            dyn_image,
            true,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        FetchTile(image)
    }
}

fn spawn_tile(
    mut events: EventReader<FetchTile>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for tile in events.read() {
        // TODO .clone() not ideal
        let texture_handle = asset_server.add(tile.0.clone());

        commands.spawn(SpriteBundle {
            texture: texture_handle,
            ..default()
        });
    }
}

fn setup(mut commands: Commands, mut bevyreq: BevyReqwest) {
    commands.spawn(Camera2dBundle::default());

    let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
    let server_url =
        "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";
    let zoom = 1;
    let x = 1;
    let y = 1;

    let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

    let request = bevyreq
        .client()
        .get(full_url)
        .build()
        .expect("Failed to build request");
    bevyreq.send(request, On::send_event::<FetchTile>())

}

fn rotate(mut query: Query<&mut Transform, With<Sprite>>, time: Res<Time>) {
    for mut bevy in &mut query {
        bevy.rotate_local_z(-time.delta_seconds());
    }
}
