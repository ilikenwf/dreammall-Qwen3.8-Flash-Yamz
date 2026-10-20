//! The bevy side: player, camera, interactions, mutation, wins, flight,
//! procedural audio playback, UI. All world geometry comes from `world.rs`;
//! all pure math from the `dreammall` lib crate.
use crate::shared::*;
use crate::world::{self, ObstacleBuildTick, WorldMats};
use bevy::audio::{AudioSinkPlayback, PlaybackMode, Volume};
use bevy::ecs::message::MessageWriter;
use bevy::image::ImageSampler;
use bevy::input::mouse::MouseMotion;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use dreammall::game::*;
use dreammall::synth;
use dreammall::texturegen;

// ---------------------------------------------------------------- assets ----

fn hsv_rgba(h: f64, s: f64, v: f64) -> Color {
    let h = h - (h as i64) as f64;
    let i = (h * 6.0).floor() as i64;
    let f = h * 6.0 - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color::srgb(r as f32, g as f32, b as f32)
}

fn plain(mats: &mut ResMut<Assets<StandardMaterial>>, m: StandardMaterial) -> Handle<StandardMaterial> {
    mats.add(m)
}

fn textured(
    images: &mut ResMut<Assets<Image>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    rgba: (u32, u32, Vec<u8>),
    base: Color,
    emissive: f32,
) -> Handle<StandardMaterial> {
    let (w, h, data) = rgba;
    world::mat_tex(images, mats, w, h, data, base, emissive)
}

/// nearest-sampled textured material (pixel art)
fn pixel_mat(
    images: &mut ResMut<Assets<Image>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    rgba: (u32, u32, Vec<u8>),
    emissive: f32,
) -> Handle<StandardMaterial> {
    let (w, h, data) = rgba;
    let img = world::tex_rgba(images, w, h, data, true);
    if let Some(mut im) = images.get_mut(&img) {
        im.sampler = ImageSampler::nearest();
    }
    let mut m = StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(img),
        perceptual_roughness: 1.0,
        unlit: true,
        ..default()
    };
    if emissive > 0.0 {
        m.emissive = LinearRgba::new(emissive, emissive, emissive, 1.0);
    }
    mats.add(m)
}

fn build_mats(
    images: &mut ResMut<Assets<Image>>,
    smats: &mut ResMut<Assets<StandardMaterial>>,
    seed: u64,
) -> Mats {
    let floor = textured(images, smats, (256, 256, texturegen::floor_tile(seed)), Color::WHITE, 0.0);
    let ceil = textured(images, smats, (256, 256, texturegen::ceil_tile(seed)), Color::WHITE, 0.0);
    let water = textured(images, smats, (128, 128, texturegen::water(seed)), Color::WHITE, 0.55);
    let metal = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.72, 0.72, 0.78),
            metallic: 0.85,
            perceptual_roughness: 0.35,
            ..default()
        },
    );
    let glass = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgba(0.75, 0.85, 0.95, 0.12),
            perceptual_roughness: 0.08,
            alpha_mode: AlphaMode::Blend,
            // faint cool self-glow so the panel never reads as a black box,
            // and no fog tint: DistanceFog would darken it against the void
            emissive: LinearRgba::new(0.05, 0.07, 0.11, 1.0),
            fog_enabled: false,
            ..default()
        },
    );
    let mirror = {
        let data = texturegen::sunset(seed);
        let img = world::tex_rgba(images, 256, 256, data, true);
        let m = smats.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(img.clone()),
            emissive: LinearRgba::new(0.85, 0.85, 0.85, 1.0),
            emissive_texture: Some(img.clone()),
            perceptual_roughness: 0.4,
            ..default()
        });
        m
    };
    let nyan = pixel_mat(images, smats, (120, 40, texturegen::nyancat()), 0.7);
    let marble = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.93, 0.91, 0.9),
            perceptual_roughness: 0.25,
            ..default()
        },
    );
    let marble_f = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.93, 0.91, 0.9),
            perceptual_roughness: 0.25,
            emissive: LinearRgba::new(0.06, 0.06, 0.065, 1.0),
            ..default()
        },
    );
    let gold = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(1.0, 0.78, 0.25),
            metallic: 1.0,
            perceptual_roughness: 0.25,
            emissive: LinearRgba::new(0.5, 0.35, 0.05, 1.0),
            ..default()
        },
    );
    let strip = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(1.0, 0.92, 0.96),
            emissive: LinearRgba::new(3.0, 2.4, 2.7, 1.0),
            ..default()
        },
    );
    let glow = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.98),
            emissive: LinearRgba::new(2.0, 1.1, 1.8, 1.0),
            ..default()
        },
    );
    let mut signs = Vec::new();
    for name in SHOP_NAMES.iter() {
        signs.push(textured(images, smats, (256, 64, texturegen::sign_tex(seed, name)), Color::WHITE, 0.6));
    }
    let mut ads = Vec::new();
    for i in 0..12 {
        let name = SHOP_NAMES[i];
        let slogan = SLOGANS[i % SLOGANS.len()];
        ads.push(textured(images, smats, (256, 256, texturegen::ad_poster(seed, name, slogan)), Color::WHITE, 0.25));
    }
    let bench = (0..4)
        .map(|i| {
            plain(
                smats,
                StandardMaterial {
                    base_color: hsv_rgba(i as f64 / 4.0 + 0.55, 0.35, 0.85),
                    perceptual_roughness: 0.6,
                    ..default()
                },
            )
        })
        .collect();
    let plant = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.2, 0.55, 0.3),
            perceptual_roughness: 0.8,
            ..default()
        },
    );
    let pot = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.65, 0.35, 0.25),
            perceptual_roughness: 0.8,
            ..default()
        },
    );
    let bun = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.87, 0.68, 0.4),
            perceptual_roughness: 0.7,
            ..default()
        },
    );
    let sausage = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.7, 0.22, 0.15),
            perceptual_roughness: 0.6,
            ..default()
        },
    );
    let mustard = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.95, 0.8, 0.1),
            perceptual_roughness: 0.5,
            ..default()
        },
    );
    let plate = plain(
        smats,
        StandardMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.95),
            perceptual_roughness: 0.3,
            ..default()
        },
    );
    Mats {
        floor,
        ceil,
        water,
        metal,
        glass,
        mirror,
        nyan,
        marble,
        marble_f,
        gold,
        strip,
        glow,
        signs,
        ads,
        bench,
        plant,
        pot,
        bun,
        sausage,
        mustard,
        plate,
    }
}

fn build_audio(sources: &mut ResMut<Assets<AudioSource>>) -> (Sfx, Music, Handle<AudioSource>, Handle<AudioSource>) {
    let mut add = |bytes: Vec<u8>| sources.add(AudioSource { bytes: bytes.into() });
    let sfx = Sfx {
        pickup: add(synth::sfx_pickup()),
        throw_: add(synth::sfx_throw()),
        chime: add(synth::sfx_win_chime()),
        munch: add(synth::sfx_munch()),
        riser: add(synth::sfx_riser()),
        glitch: add(synth::sfx_glitch()),
    };
    let foot_l = add(synth::sfx_footstep(true));
    let foot_r = add(synth::sfx_footstep(false));
    let set = synth::muzak_set();
    let durs = set
        .iter()
        .map(|b| (b.len() - 44) as f32 / (44100.0 * 2.0 * 2.0))
        .collect();
    let loops = set.into_iter().map(|b| add(b)).collect();
    let music = Music {
        loops,
        durs,
        current: 0,
        cur_ent: None,
        cur_spawn: 0.0,
        fading: Vec::new(),
    };
    (sfx, music, foot_l, foot_r)
}

// ---------------------------------------------------------------- startup ----

#[derive(Resource)]
struct NewDream(Option<u64>);

/// Animated singularity backdrop: an inverted UV sphere wrapped in a baked
/// equirectangular void panorama, centered on the mall. The vortex sits at
/// the nadir (straight below), starfield at the zenith; slow Y-rotation plus
/// 8-frame pulsing/twinkling texture loop drive the swirl.
#[derive(Resource)]
struct VoidSky {
    frames: Vec<Handle<Image>>,
    mat: Handle<StandardMaterial>,
    idx: usize,
}

#[derive(Component)]
struct VoidSphere;

/// Bake + spawn (first dream) or re-bake (later dreams, new seed) the void.
fn setup_void(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    images: &mut ResMut<Assets<Image>>,
    smats: &mut ResMut<Assets<StandardMaterial>>,
    seed: u64,
    sky: Option<&VoidSky>,
) {
    const VOID_FRAMES: usize = 8;
    let frames: Vec<Handle<Image>> = (0..VOID_FRAMES)
        .map(|i| {
            let img = world::tex_rgba(images, 512, 256, texturegen::void_frame(seed, i as f32 / VOID_FRAMES as f32), true);
            img
        })
        .collect();
    match sky {
        Some(sky) => {
            // keep the same material handle (the sphere keeps referencing
            // it); just repoint its texture at the fresh bake
            if let Some(mut m) = smats.get_mut(&sky.mat) {
                m.emissive_texture = Some(frames[0].clone());
            }
            commands.insert_resource(VoidSky {
                frames,
                mat: sky.mat.clone(),
                idx: 0,
            });
        }
        None => {
            let mat = smats.add(StandardMaterial {
                // unlit emissive backdrop: black albedo so lights never
                // touch it, fog off so the vortex glow survives DistanceFog
                base_color: Color::BLACK,
                emissive: LinearRgba::new(2.3, 2.3, 2.3, 1.0),
                emissive_texture: Some(frames[0].clone()),
                fog_enabled: false,
                cull_mode: Some(bevy::render::render_resource::Face::Front),
                ..default()
            });
            // UV sphere so the panorama maps; poles on local Z, rotated so
            // the zenith row lands up and the vortex (last row) hangs below
            let mesh = meshes.add(Sphere::new(400.0).mesh().uv(64, 32));
            commands.spawn((
                VoidSphere,
                Name::new("void-sphere"),
                Mesh3d(mesh),
                MeshMaterial3d(mat.clone()),
                Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
            commands.insert_resource(VoidSky {
                frames,
                mat,
                idx: 0,
            });
        }
    }
}

/// Cycle the pulsing frames and spin the vortex slowly about world Y.
fn void_anim(
    time: Res<Time>,
    mut sky: ResMut<VoidSky>,
    mut smats: ResMut<Assets<StandardMaterial>>,
    mut q: Query<&mut Transform, With<VoidSphere>>,
    cam: Query<&Transform, (With<Cam>, Without<VoidSphere>)>,
) {
    let t = time.elapsed_secs();
    let idx = ((t / 0.5) as usize) % sky.frames.len();
    if idx != sky.idx {
        sky.idx = idx;
        if let Some(mut m) = smats.get_mut(&sky.mat) {
            m.emissive_texture = Some(sky.frames[idx].clone());
        }
    }
    // one rotation per 50 s; pole axis is local Z after the -90° X tilt, so
    // post-multiplying Rz spins the panorama around world Y
    let spin = t * (std::f32::consts::TAU / 50.0);
    let cxz = cam.single().map(|c| c.translation).unwrap_or(Vec3::ZERO);
    for mut tf in q.iter_mut() {
        tf.translation = Vec3::new(cxz.x, 0.0, cxz.z);
        tf.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z(spin);
    }
}

fn setup_dream(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut smats: ResMut<Assets<StandardMaterial>>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut meshes: ResMut<Assets<Mesh>>,
    sky: Option<Res<VoidSky>>,
) {
    let seed = 0xC0FFEE ^ (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x51A7)
        & 0xFFFF_FFFF_0000);
    let mats = build_mats(&mut images, &mut smats, seed);
    let (sfx, music, foot_l, foot_r) = build_audio(&mut sources);
    commands.insert_resource(mats);
    commands.insert_resource(sfx);
    commands.insert_resource(music);
    commands.insert_resource(WorldMats::default());
    commands.insert_resource(NewDream(Some(seed)));
    commands.insert_resource(Run::new(seed, foot_l, foot_r));
    commands.insert_resource(ClearColor(Color::srgb(0.03, 0.01, 0.06)));
    setup_void(
        &mut commands,
        &mut meshes,
        &mut images,
        &mut smats,
        seed,
        sky.as_deref(),
    );
    spawn_ui_root(&mut commands, &mut images);
}

fn spawn_ui_root(commands: &mut Commands, images: &mut ResMut<Assets<Image>>) {
    let vig = world::tex_rgba(images, 256, 256, texturegen::vignette(), true);
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_children(|ui| {
            // vignette overlay
            ui.spawn((
                ImageNode {
                    image: vig,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                FocusPolicy::Pass,
            ));
            // black fade overlay
            ui.spawn((
                Name::new("fade"),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                FadeNode,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                FocusPolicy::Pass,
                GlobalZIndex(40),
            ));
            // prompt line (bottom center)
            ui.spawn((
                Name::new("prompt"),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(90.0),
                    left: Val::Percent(0.0),
                    right: Val::Percent(0.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                FocusPolicy::Pass,
                GlobalZIndex(41),
            ))
            .with_child((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.9, 0.97, 0.95)),
            ));
            // toast (top center)
            ui.spawn((
                Name::new("toast"),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(60.0),
                    left: Val::Percent(0.0),
                    right: Val::Percent(0.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                FocusPolicy::Pass,
                GlobalZIndex(41),
            ))
            .with_child((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(22.0),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.85, 0.95, 0.9)),
            ));
            // controls hint (bottom left, dim)
            ui.spawn((
                Name::new("hint"),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(16.0),
                    left: Val::Px(18.0),
                    ..default()
                },
                FocusPolicy::Pass,
                GlobalZIndex(41),
            ))
            .with_child((
                Text::new("W forward   A/D strafe   turn: mouse (click to grab) or Left/Right arrows   E interact"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 0.9, 0.97, 0.35)),
            ));
        });
}

/// build/rebuild the world when NewDream is pending
fn build_pending(
    mut commands: Commands,
    mut nd: ResMut<NewDream>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut smats: ResMut<Assets<StandardMaterial>>,
    mats: Res<Mats>,
    mut wm: ResMut<WorldMats>,
    mut obstacles: ResMut<Obstacles>,
    world: Query<Entity, With<WorldRoot>>,
    run: Option<Res<Run>>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    let Some(seed) = nd.0.take() else { return };
    for e in &world {
        commands.entity(e).despawn();
    }
    // spawn or reset player+camera at the start pose (facing forward tangent).
    // Player is NOT a child of WorldRoot, so it survives world respawn.
    // boundary between sections: the ring midline is clear of center props (fountains sit at section centers)
    let (sx, sz) = arc_point(0.0, R_MID);
    let start = Transform::from_xyz(sx, EYE_H, sz);
    let heading = 90.0f32; // tangent at angle 0 = +X
    match players.single_mut() {
        Ok(mut ptf) => *ptf = start,
        Err(_) => {
            commands.spawn((
                Player,
                Name::new("player"),
                // camera child gets Visibility from the render plugin; give
                // the parent the full inheritance triple so the camera's
                // visibility chain is well-formed (silences B0004)
                Visibility::Visible,
                InheritedVisibility::VISIBLE,
                ViewVisibility::default(),
                start,
                children![(
                    Cam,
                    Camera3d::default(),
                    DistanceFog::default(),
                    // dim dream-ambient: surfaces beyond the point-light
                    // falloff still read as dim walls, not pitch-black void
                    AmbientLight {
                        color: Color::srgb(0.55, 0.45, 0.75),
                        brightness: 55.0,
                        affects_lightmapped_meshes: false,
                    },
                    bevy::camera::visibility::NoCpuCulling,
                )],
            ));
        }
    }
    let (fl, fr) = run
        .map(|r| (r.foot_l.clone(), r.foot_r.clone()))
        .unwrap_or_default();
    let mut r = Run::new(seed, fl, fr);
    r.fade = 1.0; // fade in from black
    // spawn at angle 0 facing the forward tangent (tangent at 0 = +X = heading 90)
    r.theta_prev = 0.0;
    r.heading = heading;
    commands.insert_resource(r);
    *LAP.lock().unwrap() = 0.0;
    world::build_world(
        &mut commands,
        &mut meshes,
        &mut images,
        &mut smats,
        &mats,
        &mut obstacles,
        &mut wm,
        seed,
    );
}

// ---------------------------------------------------------------- player ----

fn player_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    _mouse: Res<ButtonInput<MouseButton>>,
    mut mraw: MessageReader<MouseMotion>,
    mut run: ResMut<Run>,
    mut q: Query<&mut Transform, With<Player>>,
    mut cam: Query<&mut Transform, (With<Cam>, Without<Player>)>,
    mut fog: Query<&mut DistanceFog>,
    _sfx: Res<Sfx>,
    mut commands: Commands,
    obstacles: Res<Obstacles>,
) {
    let dt = time.delta_secs().min(0.05);
    run.clock += dt;

    // mouse look (only while grabbed and not on the ending screen)
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    if run.locked {
        for m in mraw.read() {
            dx += m.delta.x;
            dy += m.delta.y;
        }
    } else {
        mraw.clear();
    }
    // arrow-key turn fallback so control never depends on pointer grab
    if run.mode == Mode::Walking {
        if keys.pressed(KeyCode::ArrowLeft) {
            run.heading += 100.0 * dt;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            run.heading -= 100.0 * dt;
        }
    }
    // right-hand turn = decreasing heading (fwd = (sin h, 0, cos h))
    run.heading -= dx * 0.12;
    run.pitch = (run.pitch - dy * 0.12).clamp(-85.0, 85.0);
    // track manual looking so the dream can guide the gaze when idle
    if dx != 0.0
        || dy != 0.0
        || keys.pressed(KeyCode::ArrowLeft)
        || keys.pressed(KeyCode::ArrowRight)
    {
        run.look_idle = 0.0;
    } else {
        run.look_idle += dt;
    }

    let Ok(mut tf) = q.single_mut() else { return };

    if run.mode == Mode::Walking {
        let mut fwd = 0.0f32;
        let mut strafe = 0.0f32;
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            fwd += 1.0;
        }
        // NOTE: no backward walking, ever. ArrowDown is deliberately unwired.
        if keys.pressed(KeyCode::KeyA) {
            strafe -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            strafe += 1.0;
        }
        let moving = fwd != 0.0 || strafe != 0.0;
        if moving {
            run.idle = 0.0;
        } else {
            run.idle += dt;
        }
        let h = run.heading.to_radians();
        let f = Vec3::new(h.sin(), 0.0, h.cos());
        let rt = Vec3::new(-h.cos(), 0.0, h.sin());
        let speed = 3.4;
        let mut vel = (f * fwd + rt * strafe).normalize_or_zero() * speed;
        // the mall guides you: pure forward walking follows the ring curve,
        // so holding W forever tracks the corridor instead of the outer wall
        if fwd > 0.0 && strafe == 0.0 {
            let q = tf.translation;
            let th = q.x.atan2(q.z);
            let t = Vec3::new(th.cos(), 0.0, -th.sin());
            vel = vel.lerp(t * speed, 1.0 - (-2.5 * dt).exp());
        }
        // gentle pressure to keep walking
        if run.idle > 15.0 {
            let q = tf.translation;
            let th = q.x.atan2(q.z);
            vel += Vec3::new(th.cos(), 0.0, -th.sin()) * 0.9;
        }
        // the mall guides you: with no recent look input, the gaze eases to
        // the corridor tangent so forward walking faces forward; any mouse or
        // arrow input above overrides instantly (look_idle resets)
        if (fwd > 0.0 || run.idle > 15.0) && strafe == 0.0 && run.look_idle > 1.5 {
            let th = tf.translation.x.atan2(tf.translation.z); // radians
            let want = th.cos().atan2(-th.sin()).to_degrees();
            run.heading += angle_delta(run.heading, want) * (1.0 - (-1.8 * dt).exp());
        }
        let mut p = tf.translation;
        p.x += vel.x * dt;
        p.z += vel.z * dt;
        // obstacle push-out
        for (c, rad) in obstacles.circles.iter() {
            let d = Vec2::new(p.x - c.x, p.z - c.y);
            let dl = d.length();
            if dl < *rad + 0.35 && dl > 1e-4 {
                let push = *rad + 0.35 - dl;
                p.x += d.x / dl * push;
                p.z += d.y / dl * push;
            }
        }
        // ring clamp LAST: obstacle push-out may shove the player into a
        // wall band; the walkable ring is a hard constraint
        let r = (p.x * p.x + p.z * p.z).sqrt().max(1e-4);
        if r < WALK_IN || r > WALK_OUT {
            let rc = r.clamp(WALK_IN, WALK_OUT);
            p.x *= rc / r;
            p.z *= rc / r;
        }
        // footstep audio by distance
        let step = vel.length() * dt;
        run.dist_since_step += step;
        if run.dist_since_step > 2.0 && moving {
            run.dist_since_step = 0.0;
            run.step_flip = !run.step_flip;
            let hnd = if run.step_flip { run.foot_l.clone() } else { run.foot_r.clone() };
            commands.spawn((
                AudioPlayer(hnd),
                PlaybackSettings {
                    mode: PlaybackMode::Despawn,
                    volume: Volume::Linear(0.5),
                    ..default()
                },
                Transform::from_translation(p),
            ));
        }
        tf.translation = p;
        // lap accumulation: theta = atan2(x,z); forward increases theta
        let theta = p.x.atan2(p.z).to_degrees();
        let delta = angle_delta(run.theta_prev, theta);
        if delta > 0.0 {
            *LAP.lock().unwrap() += delta;
        }
        run.theta_prev = theta;
        {
            let mut lap = LAP.lock().unwrap();
            if *lap >= 360.0 {
                *lap -= 360.0;
                run.pass += 1;
                run.mutate_flag = true;
            }
        }
        // fog normal
        if let Ok(mut f) = fog.single_mut() {
            f.color = Color::srgb(0.10, 0.04, 0.16);
            f.falloff = FogFalloff::Exponential { density: 0.014 };
        }
    } else {
        // ---- flight: free 6DOF, no collision ----
        let h = run.heading.to_radians();
        let p = run.pitch.to_radians();
        let fwd = Vec3::new(
            h.sin() * p.cos(),
            p.sin(),
            h.cos() * p.cos(),
        );
        let rt = Vec3::new(-h.cos(), 0.0, h.sin());
        let mut vel = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            vel += fwd;
        }
        if keys.pressed(KeyCode::KeyS) {
            vel -= fwd;
        }
        if keys.pressed(KeyCode::KeyA) {
            vel -= rt;
        }
        if keys.pressed(KeyCode::KeyD) {
            vel += rt;
        }
        if keys.pressed(KeyCode::Space) {
            vel.y += 1.0;
        }
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            vel.y -= 1.0;
        }
        if vel.length_squared() > 0.0 {
            tf.translation += vel.normalize() * 6.5 * dt;
        }
        if keys.pressed(KeyCode::KeyQ) {
            run.roll += 60.0 * dt;
        }
        if keys.pressed(KeyCode::KeyE) {
            run.roll -= 60.0 * dt;
        }
        if let Ok(mut f) = fog.single_mut() {
            f.color = Color::srgb(0.6, 0.08, 0.52);
            f.falloff = FogFalloff::Exponential { density: 0.006 };
        }
    }

    // camera: child of player? player root holds position; camera local rot = pitch/roll
    if let Ok(mut ctf) = cam.single_mut() {
        ctf.rotation = Quat::from_rotation_z(run.roll.to_radians())
            * Quat::from_rotation_x(run.pitch.to_radians());
    }
    // bevy cameras look down local -Z: +180 deg aligns view with heading
    tf.rotation = Quat::from_rotation_y((run.heading + 180.0).to_radians());
}

// camera transform + bobbing handled here (child of player root)
fn camera_rig(
    time: Res<Time>,
    run: Res<Run>,
    mut cam: Query<&mut Transform, (With<Cam>, Without<Player>)>,
) {
    let Ok(mut ctf) = cam.single_mut() else { return };
    let bob = if run.mode == Mode::Walking && (run.idle < 0.1) {
        (time.elapsed_secs() * 9.0).sin() * 0.03
    } else {
        0.0
    };
    ctf.translation = Vec3::new(0.0, bob, 0.0);
}

// ---------------------------------------------------------------- cursor ----

fn cursor_ctrl(
    mut co: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    gs: Res<State<GS>>,
    mut run: ResMut<Run>,
) {
    let Ok(mut co) = co.single_mut() else { return };
    if *gs.get() != GS::Dreaming {
        if run.locked {
            co.grab_mode = CursorGrabMode::None;
            run.locked = false;
        }
        return;
    }
    // click to grab; Escape releases (but does NOT quit - quit is only the ending screen)
    if mouse.just_pressed(MouseButton::Left) && !run.locked {
        co.grab_mode = CursorGrabMode::Locked;
        run.locked = true;
    }
    if keys.just_pressed(KeyCode::Escape) && run.locked {
        co.grab_mode = CursorGrabMode::None;
        run.locked = false;
    }
}

// ------------------------------------------------------------------- wins ----

fn interactions(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut run: ResMut<Run>,
    players: Query<&Transform, With<Player>>,
    penny: Query<(Entity, &GlobalTransform), With<Penny>>,
    hotdog: Query<&GlobalTransform, With<Hotdog>>,
    fountains: Query<&Fountain>,
    mut commands: Commands,
    sfx: Res<Sfx>,
) {
    let _dt = time.delta_secs().min(0.05);
    let Ok(ptf) = players.single() else { return };
    let ppos = ptf.translation;
    run.prompt = None;
    if run.won.is_some() {
        return; // the dream is over; no prompts or interactions on the way out
    }

    let press_e = keys.just_pressed(KeyCode::KeyE);
    // penny pickup / throw
    if !run.has_penny && run.penny_spawned {
        for (e, t) in &penny {
            let d = t.translation().distance(ppos);
            if d < 2.2 {
                run.prompt = Some("PICK UP THE PENNY  [E]".into());
                if press_e {
                    run.has_penny = true;
                    commands.entity(e).despawn();
                    commands.spawn((
                        AudioPlayer(sfx.pickup.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                    toast(&mut run, "YOU TAKE THE PENNY. IT IS WARM.");
                }
            }
        }
    } else if run.has_penny {
        for f in &fountains {
            let d = Vec2::new(f.center.x - ppos.x, f.center.z - ppos.z).length();
            if d < 4.0 && ppos.y < 2.5 {
                run.prompt = Some("THROW THE PENNY  [E]".into());
                if press_e {
                    run.has_penny = false;
                    commands.spawn((
                        AudioPlayer(sfx.throw_.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                    trigger_win(&mut run, WinKind::Water, &sfx);
                }
            }
        }
    }

    // hotdog pickup / eat
    if run.hotdog_spawned {
    for t in &hotdog {
        if t.translation().distance(ppos) < 2.0 {
            if !run.has_hotdog {
                run.prompt = Some("TAKE THE HOTDOG  [E]".into());
                if press_e {
                    run.has_hotdog = true;
                    commands.spawn((
                        AudioPlayer(sfx.pickup.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                    toast(&mut run, "IT IS STILL HOT. IT SHOULD NOT BE.");
                }
            } else if !run.hotdog_eaten {
                run.prompt = Some("EAT THE HOTDOG  [E]".into());
                if press_e {
                    run.hotdog_eaten = true;
                    commands.spawn((
                        AudioPlayer(sfx.munch.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            ..default()
                        },
                    ));
                    // enter flight
                    run.mode = Mode::Flying;
                    run.roll = 0.0;
                    toast(&mut run, "YOU ARE LIGHT. SPACE RISES. SHIFT FALLS.");
                    commands.spawn((
                        AudioPlayer(sfx.riser.clone()),
                        PlaybackSettings {
                            mode: PlaybackMode::Despawn,
                            volume: Volume::Linear(0.8),
                            ..default()
                        },
                    ));
                }
            }
        }
    }
    }

    // flight wins: rise past y>55, or dive into fountain water
    if run.mode == Mode::Flying && run.won.is_none() {
        if ppos.y > 55.0 {
            trigger_win(&mut run, WinKind::Flight, &sfx);
        } else if ppos.y < 1.0 && ppos.y > 0.2 {
            for f in &fountains {
                let d = Vec2::new(f.center.x - ppos.x, f.center.z - ppos.z).length();
                if d < 4.0 {
                    trigger_win(&mut run, WinKind::Water, &sfx);
                }
            }
        }
    }

    // idle pressure toast
    if run.idle > 6.0 && run.toast.is_none() && run.won.is_none() && run.mode == Mode::Walking {
        let idx = (run.pass as usize + run.idle as usize) % TOASTS.len();
        toast(&mut run, TOASTS[idx]);
    }
}

static LAP: std::sync::Mutex<f32> = std::sync::Mutex::new(0.0);

fn trigger_win(run: &mut Run, kind: WinKind, sfx: &Sfx) {
    if run.won.is_some() {
        return;
    }
    run.won = Some(kind);
    run.won_timer = 0.0;
    commands_spawn_chime(sfx);
}

fn commands_spawn_chime(sfx: &Sfx) {
    // chime is spawned by win_fade via Commands; store request instead
    WIN_CHIME.with(|c| *c.borrow_mut() = Some(sfx.chime.clone()));
}

thread_local! {
    static WIN_CHIME: std::cell::RefCell<Option<Handle<AudioSource>>> = const { std::cell::RefCell::new(None) };
    static OBSTACLES_TL: std::cell::RefCell<Vec<(Vec2, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn win_fade(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut commands: Commands,
    mut next: ResMut<NextState<GS>>,
    gs: Res<State<GS>>,
) {
    let dt = time.delta_secs().min(0.05);
    // fade in from black on fresh dream
    if run.won.is_none() && run.fade > 0.0 {
        run.fade = (run.fade - dt / 1.2).max(0.0);
    }
    if run.won.is_some() {
        run.won_timer += dt;
        run.fade = (run.won_timer / 1.5).min(1.0);
        if run.won_timer > 1.5 && *gs.get() == GS::Dreaming {
            run.fade = 1.0;
            next.set(GS::Ended);
        }
    }
    // apply fade + handle pending chime
    if let Some(chime) = WIN_CHIME.with(|c| c.borrow_mut().take()) {
        commands.spawn((
            AudioPlayer(chime),
            PlaybackSettings {
                mode: PlaybackMode::Despawn,
                volume: Volume::Linear(0.9),
                ..default()
            },
        ));
    }
}

// simpler: dedicated fade applier
fn fade_apply(run: Res<Run>, mut q: Query<&mut BackgroundColor, With<FadeNode>>) {
    for mut bg in q.iter_mut() {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, run.fade);
    }
}

#[derive(Component)]
struct FadeNode;

// ---------------------------------------------------------------- mutation ----

fn mutation(
    mut run: ResMut<Run>,
    mut wm: ResMut<WorldMats>,
    mut mats_q: Query<(&Sec, &Role, Option<&Slot>, &mut Transform, Option<&mut MeshMaterial3d<StandardMaterial>>, Option<&mut Visibility>, Option<&mut PointLight>), With<Role>>,
    // one query: ECS cannot prove With<Hotdog> and With<Penny> disjoint,
    // so two &mut Visibility queries would conflict at schedule init
    mut reveal: Query<(&mut Visibility, Has<Penny>), (Or<(With<Penny>, With<Hotdog>)>, Without<Role>)>,
    mats: Res<Mats>,
    sfx: Res<Sfx>,
    mut commands: Commands,
) {
    if !std::mem::take(&mut run.mutate_flag) {
        return;
    }
    let seed = run.seed;
    let pass = run.pass;
    commands.spawn((
        AudioPlayer(sfx.glitch.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(0.6),
            ..default()
        },
    ));
    toast(&mut run, "YOU HAVE BEEN HERE BEFORE. YOU HAVEN'T.");
    let mut side_changes = 0u32;
    for (sec, m, slot, mut tf, mmat, vis, light) in mats_q.iter_mut() {
        let i = sec.0;
        let h = |k: u64| hash2(hash2(seed, (pass as u64) * 131 + i as u64), k);
        match *m {
            Role::Wall => {
                let v = (h(3) % 3) as usize;
                if wm.cur_variant[i] != v {
                    wm.cur_variant[i] = v;
                    if let Some(mut mm) = mmat {
                        *mm = MeshMaterial3d(wm.wall_variants[i][v].clone());
                    }
                }
            }
            Role::Sign => {
                let s = slot.map(|s| s.0).unwrap_or(0);
                let ns = ((h(5) as usize + s) % 12.max(1)) % mats.signs.len();
                if let Some(mut mm) = mmat {
                    *mm = MeshMaterial3d(mats.signs[ns].clone());
                }
                if h(7) % 4 == 0 {
                    tf.scale.x = -tf.scale.x;
                }
            }
            Role::Ad => {
                let s = slot.map(|s| s.0).unwrap_or(0);
                let ns = ((h(11) as usize + s * 3) % 12.max(1)) % mats.ads.len();
                if let Some(mut mm) = mmat {
                    *mm = MeshMaterial3d(mats.ads[ns].clone());
                }
            }
            Role::Glass => {}
            Role::Light => {
                if let Some(mut pl) = light {
                    let hue = (h(13) % 1000) as f32 / 1000.0;
                    pl.color = hsv_rgba(hue as f64, 0.45, 1.0);
                }
            }
            Role::Side => {
                if side_changes < 6 && h(17) % 3 == 0 {
                    // mirror radius about midline, clamp inside corridor
                    let x = tf.translation.x;
                    let z = tf.translation.z;
                    let r = (x * x + z * z).sqrt().max(1e-4);
                    let nr = (2.0 * MID - r).clamp(R_IN + 1.4, R_OUT - 1.4);
                    tf.translation.x = x * nr / r;
                    tf.translation.z = z * nr / r;
                    side_changes += 1;
                }
            }
            Role::Water => {}
        }
        // hotdog reveal handled below via vis
        let _ = &vis;
    }
    // seeded reveals: hotdog on pass>=1, penny on pass 1 or 2 (spread the
    // two dreams apart)
    let reveal_pass = 1 + (hash2(seed, 101) % 2) as u32;
    let want_hotdog = pass >= 1 && !run.hotdog_spawned;
    let want_penny = pass >= reveal_pass && !run.penny_spawned;
    if want_hotdog || want_penny {
        for (mut v, is_penny) in reveal.iter_mut() {
            if (is_penny && want_penny) || (!is_penny && want_hotdog) {
                *v = Visibility::Visible;
            }
        }
        if want_hotdog {
            run.hotdog_spawned = true;
            toast(&mut run, "SOMETHING NEW IS ON A TABLE SOMEWHERE.");
        }
        if want_penny {
            run.penny_spawned = true;
            toast(&mut run, "SOMETHING GLINTS ON A TABLE SOMEWHERE.");
        }
    }
    world::rebuild_obstacles(&mut commands);
}

// ---------------------------------------------------------------- obstacles ----

fn rebuild_obstacles_sys(
    mut commands: Commands,
    tick: Res<ObstacleBuildTick>,
    mut obstacles: ResMut<Obstacles>,
    props: Query<(&Prop, &Obs, &SectionRoot), With<Obs>>,
    roots: Query<(&SectionRoot, &GlobalTransform)>,
    hotdogs: Query<&Transform, With<Hotdog>>,
    run: Res<Run>,
) {
    let _ = tick;
    obstacles.clear();
    for (root, gt) in &roots {
        let _ = (root, gt);
    }
    // world positions: section roots are rotated; use stored Prop r/off + section center
    for (prop, obs, sec) in &props {
        let c = section_center(sec.id);
        let (x, z) = arc_point(c + prop.off, prop.r);
        obstacles.add(x, z, obs.0);
    }
    // fountain centers block
    for f in hotdogs.iter() {
        let _ = f;
    }
    let _ = run;
    let _ = &mut commands;
}

// ------------------------------------------------------------------ music ----

fn music_sched(
    time: Res<Time>,
    run: ResMut<Run>,
    mut music: ResMut<Music>,
    mut commands: Commands,
    mut sources: ResMut<Assets<AudioSource>>,
    mut sinks: Query<(&mut AudioSink, Entity)>,
) {
    let _dt = time.delta_secs().min(0.05);
    let clock = run.clock;
    // start first loop
    if music.cur_ent.is_none() && !music.loops.is_empty() {
        let e = spawn_loop(&mut commands, &music, 0.45);
        music.cur_ent = Some(e);
        music.cur_spawn = clock;
    }
    // schedule next loop variant 1.5s before current ends
    if let (Some(ent), Some(dur)) = (music.cur_ent, music.durs.get(music.current)) {
        let _ = ent;
        let elapsed = clock - music.cur_spawn;
        if elapsed > dur - 1.5 {
            // pick different variant
            let n = music.loops.len();
            let mut next =
                (music.current + 1 + (rand_u32(clock.to_bits() as u64) as usize) % (n - 1)) % n;
            if next == music.current {
                next = (next + 1) % n;
            }
            music.current = next;
            let e = spawn_loop(&mut commands, &music, 0.45);
            music.cur_ent = Some(e);
            music.cur_spawn = clock;
            // fade old one: find all audio sinks not in the new spawn... simplest: fade all existing
            for (sink, e) in sinks.iter_mut() {
                let _ = sink;
                music.fading.push((e, clock));
            }
        }
    }
    // ramp fading sinks down
    let mut done = Vec::new();
    for (e, t0) in music.fading.iter_mut() {
        let t = (clock - *t0) / 1.5;
        if let Ok((mut sink, _)) = sinks.get_mut(*e) {
            let v = (0.45 * (1.0 - t)).max(0.0);
            sink.set_volume(Volume::Linear(v));
        }
        if t >= 1.0 {
            done.push(*e);
        }
    }
    for e in done {
        music.fading.retain(|(x, _)| *x != e);
        commands.entity(e).despawn();
    }
    let _ = &mut sources;
}

fn rand_u32(x: u64) -> u32 {
    let mut h = x.wrapping_mul(0x9E3779B97F4A7C15);
    h ^= h >> 31;
    h.wrapping_mul(0xBF58476D1CE4E5B9) as u32
}

fn spawn_loop(commands: &mut Commands, music: &Music, vol: f32) -> Entity {
    commands
        .spawn((
            AudioPlayer(music.loops[music.current].clone()),
            PlaybackSettings {
                mode: PlaybackMode::Once,
                volume: Volume::Linear(vol),
                ..default()
            },
        ))
        .id()
}

// ---------------------------------------------------------------- ending UI ----

fn spawn_ending_ui(
    mut commands: Commands,
    run: Res<Run>,
    winent: Res<WinShown>,
) {
    let msg = match winent.0 {
        Some(WinKind::Water) => "THE WATER TAKES YOU DOWN, WARM AND SWEET.\nYOU WOKE UP, CONGRATULATIONS.",
        Some(WinKind::Flight) => "YOU RISE THROUGH THE GLASS ROOF.\nTHE PARKING LOT IS AN OCEAN OF MIST.\nYOU WOKE UP, CONGRATULATIONS.",
        None => "YOU WOKE UP, CONGRATULATIONS.",
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(28.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.0, 0.05, 0.94)),
            GlobalZIndex(50),
            DespawnOnExit(GS::Ended),
        ))
        .with_children(|ui| {
            ui.spawn((
                Text::new(msg),
                TextFont {
                    font_size: FontSize::Px(34.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.95)),
                TextLayout {
                    justify: Justify::Center,
                    linebreak: LineBreak::WordBoundary,
                    ..default()
                },
            ));
            ui.spawn((
                Button,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(64.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.45, 0.15, 0.5)),
                SleepBtn,
            ))
            .observe(sleep_btn)
            .with_child((
                Text::new("GO BACK TO SLEEP?"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            ui.spawn((
                Button,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(64.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.25, 0.1, 0.3)),
                ExitBtn,
            ))
            .observe(exit_btn)
            .with_child((
                Text::new("EXIT"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    let _ = run;
}

#[derive(Resource, Default)]
struct WinShown(Option<WinKind>);

fn capture_win(mut w: ResMut<WinShown>, run: Res<Run>) {
    if w.0.is_none() {
        w.0 = run.won;
    }
}

fn sleep_btn(
    _tr: On<Activate>,
    mut next: ResMut<NextState<GS>>,
    mut nd: ResMut<NewDream>,
    mut w: ResMut<WinShown>,
) {
    w.0 = None;
    nd.0 = Some(0xC0FFEE ^ (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x51A7)
        & 0xFFFF_FFFF_0000));
    next.set(GS::Dreaming);
}

fn exit_btn(_tr: On<Activate>, mut writer: MessageWriter<AppExit>) {
    writer.write(AppExit::Success);
}

// ----------------------------------------------------------------- toasts ----

fn ui_update(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut texts: Query<(&Name, &Children)>,
    mut textq: Query<&mut Text>,
) {
    let dt = time.delta_secs().min(0.05);
    if let Some((_, t)) = run.toast.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            run.toast = None;
        }
    }
    let prompt = run.prompt.clone().unwrap_or_default();
    let toast = run.toast.as_ref().map(|(m, _)| m.clone()).unwrap_or_default();
    let hint: String = if run.mode == Mode::Flying {
        "WASD move   SPACE rise  SHIFT sink   mouse look   Q/E roll".into()
    } else {
        "W forward   A/D strafe   turn: mouse (click to grab) or Left/Right arrows   E interact".into()
    };
    for (name, kids) in texts.iter_mut() {
        let want = match name.as_str() {
            "prompt" => &prompt,
            "toast" => &toast,
            "hint" => &hint,
            _ => continue,
        };
        for k in kids.iter() {
            if let Ok(mut t) = textq.get_mut(k) {
                if t.0 != *want {
                    t.0 = want.clone();
                }
            }
        }
    }
}

/// clear stale toast/prompt text so nothing from the dream leaks onto the ending screen
fn clear_hud_on_end(
    mut run: ResMut<Run>,
    mut texts: Query<(&Name, &Children)>,
    mut textq: Query<&mut Text>,
) {
    run.toast = None;
    run.prompt = None;
    for (name, kids) in texts.iter_mut() {
        if name.as_str() == "prompt" || name.as_str() == "toast" || name.as_str() == "hint" {
            for k in kids.iter() {
                if let Ok(mut t) = textq.get_mut(k) {
                    t.0.clear();
                }
            }
        }
    }
}

// ---------------------------------------------------------------- glue sys ----

/// sync obstacles TL for hot path collision
fn obstacles_sync(obstacles: Res<Obstacles>) {
    OBSTACLES_TL.with(|o| *o.borrow_mut() = obstacles.circles.clone());
}

pub fn build(app: &mut App) {
    // Mesa's llvmpipe (26.x) advertises VK_KHR_cooperative_matrix but its ICD
    // rejects it at vkCreateDevice; wgpu's `Functionality` priority would request
    // the experimental feature and panic. The game never uses cooperative
    // matrices, so disable it unconditionally for driver robustness.
    app.add_plugins(DefaultPlugins.set(bevy::render::RenderPlugin {
        render_creation: bevy::render::settings::RenderCreation::Automatic(Box::new(
            bevy::render::settings::WgpuSettings {
                disabled_features: Some(
                    bevy::render::settings::WgpuFeatures::EXPERIMENTAL_COOPERATIVE_MATRIX,
                ),
                ..default()
            },
        )),
        ..default()
    }).set(WindowPlugin {
        primary_window: Some(Window {
            title: "DREAM MALL".into(),
            resolution: (1600, 900).into(),
            present_mode: bevy::window::PresentMode::AutoVsync,
            ..default()
        }),
        ..default()
    }));
    app.init_resource::<ObstacleBuildTick>();
    app.init_resource::<Obstacles>();
    app.init_resource::<WinShown>();
    app.init_state::<GS>();
    app.add_systems(OnEnter(GS::Dreaming), setup_dream);
    app.add_systems(
        Update,
        (
            build_pending.run_if(resource_exists::<NewDream>),
            obstacles_sync,
            player_input
                .after(build_pending)
                .before(bevy::transform::TransformSystems::Propagate),
            camera_rig.after(player_input),
            interactions.after(player_input),
            mutation.after(interactions),
            rebuild_obstacles_sys.run_if(resource_changed::<ObstacleBuildTick>),
            music_sched.after(player_input),
            ui_update,
            capture_win,
            win_fade,
            fade_apply.after(win_fade),
            nyan_anim,
            float_drift,
            fountain_anim,
            void_anim,
        )
            .run_if(in_state(GS::Dreaming)),
    );
    // cursor grab/release must also run during the ending screen so the win
    // fade releasing the grab leaves the mouse free for the UI buttons
    app.add_systems(Update, cursor_ctrl.run_if(resource_exists::<Run>));
    app.add_systems(OnEnter(GS::Ended), (capture_win2, spawn_ending_ui, clear_hud_on_end));
    // ending screen: Escape quits
    app.add_systems(Update, exit_on_escape.run_if(in_state(GS::Ended)));
}

fn exit_on_escape(keys: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Escape) {
        writer.write(AppExit::Success);
    }
}


fn capture_win2(mut w: ResMut<WinShown>, run: Res<Run>) {
    w.0 = run.won;
}

// ---------------------------------------------------------------- ambience ----

/// nyan cat swims side-to-side across its mirror, phase-offset per mirror
fn nyan_anim(time: Res<Time>, mut q: Query<(&NyanQuad, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (n, mut tf) in q.iter_mut() {
        tf.translation.x = (t * 0.8 + n.phase).sin() * 0.55;
        tf.translation.y = 0.55 + (t * 2.3 + n.phase).sin() * 0.03;
    }
}

/// surreal shapes beyond the atrium glass rotate on their seeded axes
fn float_drift(time: Res<Time>, mut q: Query<(&FloatShape, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (f, mut tf) in q.iter_mut() {
        tf.rotation = Quat::from_axis_angle(f.axis, t * f.speed);
    }
}

/// fountain water shimmers/bobs; jets pulse their height
fn fountain_anim(time: Res<Time>, mut q: Query<(Option<&WaterSurface>, Option<&WaterJet>, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (ws, wj, mut tf) in q.iter_mut() {
        if let Some(w) = ws {
            // gentle bob + scale ripple on the basin surface
            tf.translation.y = 0.72 + (t * 1.7 + w.phase).sin() * 0.015;
            let s = 1.0 + (t * 2.1 + w.phase).sin() * 0.012;
            tf.scale = Vec3::new(s, s, 1.0);
        }
        if let Some(j) = wj {
            // jet columns pulse between 40% and 130% height, anchored at the basin
            let p = 0.85 + (t * 3.1 + j.phase).sin() * 0.45;
            tf.scale.y = p;
            tf.translation.y = 0.75 + 0.4 * p;
        }
    }
}
