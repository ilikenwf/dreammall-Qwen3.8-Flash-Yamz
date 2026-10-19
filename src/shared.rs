//! Shared ECS types for the bevy binary (components/resources). Pure math lives in `game`.

use crate::game::{R_IN, R_MID, R_OUT};
use bevy::audio::Volume;
use bevy::ecs::system::Resource;
use bevy::math::Vec2;
use bevy::prelude::{Color, Component, Entity, Handle, Image, Mesh, Quat, Vec3};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{audio::AudioSource, pbr::StandardMaterial};

#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GS {
    Dreaming,
    Ended,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Walking,
    Flying,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WinKind {
    Water,
    Flight,
}

#[derive(Resource)]
pub struct Run {
    pub seed: u64,
    pub pass: u32,
    pub mode: Mode,
    pub has_penny: bool,
    pub hotdog_spawned: bool,
    pub has_hotdog: bool,
    pub hotdog_eaten: bool,
    pub idle: f32,
    pub dist_since_step: f32,
    pub step_flip: bool,
    pub heading: f32, // yaw deg; walking forward dir = (sin,0,cos)
    pub pitch: f32,
    pub roll: f32,
    pub theta_prev: f32,
    pub toast: Option<(String, f32)>,
    pub prompt: Option<String>,
    pub mutate_flag: bool,
    pub fade: f32,           // 0..1 black overlay
    pub won: Option<WinKind>, // set by win trigger; ends after fade
    pub won_timer: f32,
    pub clock: f32,
    pub foot_l: Handle<AudioSource>,
    pub foot_r: Handle<AudioSource>,
}

impl Run {
    pub fn new(seed: u64, foot_l: Handle<AudioSource>, foot_r: Handle<AudioSource>) -> Self {
        Run {
            seed,
            pass: 0,
            mode: Mode::Walking,
            has_hotdog: false,
            has_penny: false,
            hotdog_spawned: false,
            hotdog_eaten: false,
            idle: 0.0,
            dist_since_step: 0.0,
            step_flip: false,
            heading: 90.0,
            pitch: 0.0,
            roll: 0.0,
            theta_prev: 0.0,
            toast: None,
            prompt: None,
            mutate_flag: false,
            fade: 0.0,
            won: None,
            won_timer: 0.0,
            clock: 0.0,
            foot_l,
            foot_r,
        }
    }
}

pub fn toast(run: &mut Run, msg: &str) {
    run.toast = Some((msg.to_string(), 4.0));
}

/// Runtime materials, all generated procedurally at setup.
#[derive(Resource)]
pub struct Mats {
    pub floor: Handle<StandardMaterial>,
    pub ceil: Handle<StandardMaterial>,
    pub walls: Vec<Handle<StandardMaterial>>, // 3 hue variants, swapped on mutation
    pub brick: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
    pub glass: Handle<StandardMaterial>,
    pub mirror: Handle<StandardMaterial>,
    pub dark: Handle<StandardMaterial>,
    pub sunset: Handle<StandardMaterial>,
    pub nyan: Handle<StandardMaterial>,
    pub marble: Handle<StandardMaterial>,
    pub gold: Handle<StandardMaterial>,
    pub strip: Handle<StandardMaterial>, // emissive light strip
    pub signs: Vec<Handle<StandardMaterial>>, // 12
    pub ads: Vec<Handle<StandardMaterial>>, // 12
    pub bench: Vec<Handle<StandardMaterial>>, // 4 pastel
    pub plant: Handle<StandardMaterial>,
    pub pot: Handle<StandardMaterial>,
    pub bun: Handle<StandardMaterial>,
    pub sausage: Handle<StandardMaterial>,
    pub mustard: Handle<StandardMaterial>,
    pub plate: Handle<StandardMaterial>,
    pub nyan_img: Handle<Image>,
}

#[derive(Resource)]
pub struct Sfx {
    pub pickup: Handle<AudioSource>,
    pub throw_: Handle<AudioSource>,
    pub chime: Handle<AudioSource>,
    pub munch: Handle<AudioSource>,
    pub riser: Handle<AudioSource>,
    pub glitch: Handle<AudioSource>,
}

#[derive(Resource)]
pub struct Music {
    pub loops: Vec<Handle<AudioSource>>,
    pub durs: Vec<f32>,
    pub current: usize,
    pub cur_ent: Option<Entity>,
    pub cur_spawn: f32,
    pub cur_vol: Volume,
    pub fading: Vec<(Entity, f32)>, // (entity, fadeout_start_clock)
}

/// circular obstacles (world XZ) for walking collision
#[derive(Resource, Default)]
pub struct Obstacles {
    pub circles: Vec<(Vec2, f32)>,
}

impl Obstacles {
    pub fn clear(&mut self) {
        self.circles.clear();
    }
    pub fn add(&mut self, x: f32, z: f32, r: f32) {
        self.circles.push((Vec2::new(x, z), r));
    }
}

// ---- markers ----
#[derive(Component)]
pub struct WorldRoot;
/// entities tagged with owning section id (0..N_SEC)
#[derive(Component)]
pub struct Sec(pub usize);
/// mutable-by-mutation role
#[derive(Component)]
pub enum Mut {
    Wall,
    Sign,
    Ad,
    Light,
    Side, // bench/table/plant: mirror r about R_MID
    Water,
    Nyan, // nyan-cat quad inside a mirror
}
/// local arc placement spec: ring radius, angle offset from section center, height
#[derive(Component)]
pub struct Prop {
    pub r: f32,
    pub off: f32,
    pub y: f32,
    pub rot: f32, // extra Y rotation beyond rot_y(off), degrees
}
/// stable per-section slot index for hashing (signs/ads/side props)
#[derive(Component)]
pub struct Slot(pub usize);
/// collision circle radius (world XZ), rebuilt from Prop + this on mutation
#[derive(Component)]
pub struct Obs(pub f32);
#[derive(Component)]
pub struct Penny;
#[derive(Component)]
pub struct Hotdog;
#[derive(Component)]
pub struct Fountain {
    pub center: Vec3,
}
#[derive(Component)]
pub struct NyanQuad {
    pub phase: f32,
}
#[derive(Component)]
pub struct FloatShape {
    pub speed: f32,
    pub axis: Vec3,
}
#[derive(Component)]
pub struct Cam;
#[derive(Component)]
pub struct Player;
#[derive(Component)]
pub struct SleepBtn;
#[derive(Component)]
pub struct ExitBtn;

// ---- layout helpers shared by world/player ----
pub const MID: f32 = R_MID;
pub const WALK_IN: f32 = R_IN + 0.45; // walkable clamp (inner)
pub const WALK_OUT: f32 = R_OUT - 0.45; // walkable clamp (outer)

pub fn local_arc(off_deg: f32, r: f32, y: f32) -> Vec3 {
    let (x, z) = crate::game::arc_point(off_deg, r);
    Vec3::new(x, y, z)
}

/// section root transform: rotated so local angle 0 == section center direction
pub fn section_transform(i: usize) -> Transform {
    let c = crate::game::section_center(i);
    Transform::from_rotation(Quat::from_rotation_y(c.to_radians()))
}

/// shared mesh helpers used by world spawning
pub fn quad(w: f32, h: f32) -> Mesh {
    // vertical quad facing +Z in local space, uv 0..1
    let hw = w / 2.0;
    let hh = h / 2.0;
    let mut m = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList);
    m.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[-hw, -hh, 0.0], [hw, -hh, 0.0], [hw, hh, 0.0], [-hw, hh, 0.0]],
    );
    m.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    m.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![[0.0, 0.0, 1.0]; 4],
    );
    m.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    m
}

pub fn upload_image(
    assets: &mut bevy::asset::Assets<Image>,
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
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ))
}
