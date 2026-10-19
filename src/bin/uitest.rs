// Minimal: does UI render through a Camera3d under this wgpu/lavapipe env at all?
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (640, 360).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_observer(save_to_disk("/tmp/uitest.png".into()))
        .add_systems(Update, shot)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3d);
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(1.0, 0.0, 0.0)),
    ))
    .with_child((
        Text::new("HELLO UI"),
        TextFont {
            font_size: FontSize::Px(48.0),
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
    ));
}

fn shot(time: Res<Time>, mut done: Local<bool>, mut commands: Commands) {
    if !*done && time.elapsed_secs() > 2.0 {
        *done = true;
        commands.spawn(Screenshot::primary_window());
    }
}
