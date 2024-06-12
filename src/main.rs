use bevy::{
    asset::AssetMetaCheck, ecs::system::EntityCommands, prelude::*,
    render::render_asset::RenderAssetUsages,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
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
            WorldInspectorPlugin::new(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, digest_image)
        .run();
}

#[derive(Component)]
struct ImageToDigest(Image);

#[derive(Component)]
struct TilePos {
    zoom: i32,
    y: i32,
    x: i32,
}

fn setup(mut commands: Commands, mut bevyreq: BevyReqwest) {
    commands.spawn(Camera2dBundle::default());

    let token = "AAPK8175dc0aa561421eaf15ccaa1827be79lHuQeBbDSktmG6Zc3-ntUn2kaBPPCyYTcO_4y2cmWx-NRq9ta6ERQVDJPJbsqm4_";
    let server_url =
        "https://ibasemaps-api.arcgis.com/arcgis/rest/services/World_Imagery/MapServer";

    vec![(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().for_each(|(x, y)| {
        let zoom = 1;

        let full_url = format!("{server_url}/tile/{zoom}/{y}/{x}?token={token}");

        let request = bevyreq
            .client()
            .get(full_url)
            .build()
            .expect("Failed to build request");

        let entity = commands.spawn(TilePos { zoom, y, x }).id();

        bevyreq.send_using_entity(entity, request, On::target_commands_mut(parse_image));
    });
}

fn parse_image(req: &mut ListenerInput<ReqResponse>, entity_commands: &mut EntityCommands) {
    let bytes = req.body();

    let dyn_image = image::load_from_memory(bytes).expect("cant load image");

    let image = Image::from_dynamic(
        dyn_image,
        true,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    entity_commands.insert(ImageToDigest(image));
}

fn digest_image(
    mut commands: Commands,
    query: Query<(Entity, &ImageToDigest, &TilePos)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, image, tile_pos) in query.iter() {
        // TODO remove clone
        let texture_handle = asset_server.add(image.0.clone());

        let sprite_size = 400.;

        commands
            .entity(entity)
            .insert(SpriteBundle {
                texture: texture_handle,
                transform: Transform::from_xyz(
                    tile_pos.x as f32 * sprite_size,
                    -tile_pos.y as f32 * sprite_size + sprite_size / 2.,
                    0. + sprite_size / 2. - sprite_size / 2.,
                ),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(sprite_size)),
                    ..default()
                },
                ..default()
            })
            .remove::<ImageToDigest>()
            .remove::<On<ReqResponse>>();
    }
}
