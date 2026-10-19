//! Procedural mall construction: reusable sections along a ring, mutation-ready.
use crate::shared::*;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use dreammall::game::*;

pub fn tex_rgba(
    assets: &mut ResMut<Assets<Image>>,
    w: u32,
    h: u32,
    data: Vec<u8>,
    srgb: bool,
) -> Handle<Image> {
    let fmt = if srgb {
        TextureFormat::Rgba8UnormSrgb
    } else {
        TextureFormat::Rgba8Unorm
    };
    assets.add(Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        fmt,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

pub fn mat_tex(
    assets: &mut ResMut<Assets<Image>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    w: u32,
    h: u32,
    data: Vec<u8>,
    base: Color,
    emissive: f32,
) -> Handle<StandardMaterial> {
    let img = tex_rgba(assets, w, h, data, true);
    let mut m = StandardMaterial {
        base_color: base,
        base_color_texture: Some(img),
        roughness: 0.65,
        ..default()
    };
    if emissive > 0.0 {
        m.emissive = LinearRgba::from(base).to_f32_array();
        // scale emissive by factor
        m.emissive = LinearRgba {
            red: m.emissive[0] * emissive,
            green: m.emissive[1] * emissive,
            blue: m.emissive[2] * emissive,
            alpha: 1.0,
        };
        m.emissive_texture = m.base_color_texture.clone();
    }
    mats.add(m)
}

pub fn build_world(
    commands: &mut Commands,
    assets: &mut ResMut<Assets<Image>>,
    mats: &Res<Mats>,
    mut obstacles: ResMut<Obstacles>,
    seed: u64,
) {
    obstacles.clear();
    let root = commands
        .spawn((WorldRoot, Visibility default_marker()))
        .id();
    // NOTE: replaced below
    let _ = root;
}

fn default_marker() -> Visibility {
    Visibility::Inherited
}
