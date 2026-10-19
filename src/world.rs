//! Procedural mall construction: reusable sections along a ring, mutation-ready.
use crate::shared::*;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use dreammall::game::*;
use dreammall::texturegen;

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

/// textured standard material; emissive>0 scales base color into emissive
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
        perceptual_roughness: 0.65,
        ..default()
    };
    if emissive > 0.0 {
        let lc = LinearRgba::from(base);
        m.emissive = LinearRgba::new(lc.red * emissive, lc.green * emissive, lc.blue * emissive, 1.0);
        m.emissive_texture = m.base_color_texture.clone();
    }
    mats.add(m)
}

fn plane(w: f32, h: f32) -> Handle<Mesh> {
    Mesh::from(bevy::mesh::Plane3d::new(w, h)).into()
}

fn box_m(x: f32, y: f32, z: f32) -> Handle<Mesh> {
    Mesh::from(bevy::mesh::Cuboid::new(x, y, z)).into()
}

fn cyl(r: f32, h: f32) -> Handle<Mesh> {
    Mesh::from(bevy::mesh::Cylinder::new(r, h)).into()
}

fn ball(r: f32) -> Handle<Mesh> {
    Mesh::from(bevy::mesh::Sphere::new(r)).into()
}

/// per-section material instances so mutation can recolor one section only
#[derive(Resource)]
pub struct WorldMats {
    /// [section][variant] wall materials
    pub wall_variants: Vec<[Handle<StandardMaterial>; 3]>,
    pub cur_variant: Vec<usize>,
}

/// ring band mesh between r0..r1 spanning [a0,a1] degrees at height y.
/// uv u along arc (0..1), v radial; caller tiles via texture repeat.
fn band_mesh(a0: f32, a1: f32, r0: f32, r1: f32, y: f32, segments: usize) -> Handle<Mesh> {
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for s in 0..=segments {
        let t = a0 + (a1 - a0) * s as f32 / segments as f32;
        let (sn, cs) = (t.to_radians().sin(), t.to_radians().cos());
        pos.push([r0 * sn, y, r0 * cs]);
        pos.push([r1 * sn, y, r1 * cs]);
        let u = s as f32 / segments as f32;
        uv.push([u * 8.0, 0.0]);
        uv.push([u * 8.0, 1.0]);
    }
    for s in 0..segments as u32 {
        let a = s * 2;
        idx.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }
    let mut m = Mesh::new(bevy::mesh::MeshKind::TriangleList);
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.set_indices(Some(bevy::mesh::MeshIndices::U32(idx)));
    m.compute_normals();
    m.into()
}

/// vertical wall band: same as band but extruded 0..h, normal horizontal
fn wall_band(a0: f32, a1: f32, r: f32, h: f32, segments: usize) -> Handle<Mesh> {
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for s in 0..=segments {
        let t = a0 + (a1 - a0) * s as f32 / segments as f32;
        let (sn, cs) = (t.to_radians().sin(), t.to_radians().cos());
        pos.push([r * sn, 0.0, r * cs]);
        pos.push([r * sn, h, r * cs]);
        let u = s as f32 / segments as f32;
        uv.push([u * 10.0, 0.0]);
        uv.push([u * 10.0, h / 2.0]);
    }
    for s in 0..segments as u32 {
        let a = s * 2;
        idx.extend_from_slice(&[a, a + 2, a + 1, a + 1, a + 2, a + 3]);
    }
    let mut m = Mesh::new(bevy::mesh::MeshKind::TriangleList);
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.set_indices(Some(bevy::mesh::MeshIndices::U32(idx)));
    m.compute_normals();
    m.into()
}

pub fn ceil_h(i: usize, seed: u64) -> f32 {
    CEIL_BASE + (band(section_center(i) as f32, seed) - 0.5) * CEIL_VAR
}

fn spawn_section_shell(
    commands: &mut Commands,
    root: Entity,
    mats: &Mats,
    wm: &WorldMats,
    seed: u64,
    i: usize,
) {
    let kind = section_kind(i);
    let h = ceil_h(i, seed);
    let v = wm.cur_variant[i];
    let sec = commands
        .spawn((
            SectionRoot { id: i },
            Sec(i),
            section_transform(i),
        ))
        .id();
    commands.entity(root).add_child(sec);

    // floor + ceiling bands, local angles -18..18
    let floor = commands
        .spawn((
            Sec(i),
            Mesh3d(band_mesh(-18.0, 18.0, R_IN, R_OUT, 0.0, 8)),
            MeshMaterial3d(mats.floor.clone()),
        ))
        .id();
    let ceil = commands
        .spawn((
            Sec(i),
            Mesh3d(band_mesh(-18.0, 18.0, R_IN + 0.01, R_OUT - 0.01, h, 8)),
            MeshMaterial3d(mats.ceil.clone()),
        ))
        .id();
    commands.entity(sec).add_children(&[floor, ceil]);

    // outer wall
    let outer = commands
        .spawn((
            Sec(i),
            Mesh3d(wall_band(-18.0, 18.0, R_OUT, h, 8)),
            MeshMaterial3d(wm.wall_variants[i][v].clone()),
            Mut::Wall,
        ))
        .id();
    commands.entity(sec).add_child(outer);

    // inner wall: glass for atrium (void beyond), solid otherwise
    let inner_mat = if kind == SectionKind::Atrium {
        mats.glass.clone()
    } else {
        wm.wall_variants[i][v].clone()
    };
    let inner = commands
        .spawn((
            Sec(i),
            Mesh3d(wall_band(-18.0, 18.0, R_IN, h, 8)),
            MeshMaterial3d(inner_mat),
            if kind == SectionKind::Atrium {
                Mut::Ad
            } else {
                Mut::Wall
            },
        ))
        .id();
    commands.entity(sec).add_child(inner);

    // ceiling light strip
    let strip = commands
        .spawn((
            Sec(i),
            Mesh3d(band_mesh(-14.0, 14.0, R_MID - 0.5, R_MID + 0.5, h - 0.06, 6)),
            MeshMaterial3d(mats.strip.clone()),
        ))
        .id();
    commands.entity(sec).add_child(strip);

    // point light at section center
    commands.spawn((
        Sec(i),
        PointLight {
            intensity: 9000.0,
            range: 26.0,
            color: Color::srgb(1.0, 0.86, 0.92),
            ..default()
        },
        Transform::from_translation(local_arc(0.0, MID, h - 0.6)),
        Mut::Light,
    ));

    match kind {
        SectionKind::Shops => spawn_shops(commands, sec, mats, seed, i),
        SectionKind::FoodCourt => spawn_food(commands, sec, root, mats, seed, i),
        SectionKind::Fountain => spawn_fountain(commands, sec, root, mats, seed, i),
        SectionKind::Atrium => spawn_atrium(commands, sec, mats, seed, i),
    }
}

fn spawn_shops(
    commands: &mut Commands,
    sec: Entity,
    mats: &Mats,
    seed: u64,
    i: usize,
) {
    // 4 storefronts on outer wall
    let offs = [-13.5f32, -4.5, 4.5, 13.5];
    for (slot, off) in offs.iter().enumerate() {
        let gi = (i * 4 + slot) % 12;
        // window plane slightly inside wall
        let win = commands
            .spawn((
                Sec(i),
                Slot(slot),
                Mut::Sign,
                Mesh3d(plane(3.4, 2.4)),
                MeshMaterial3d(mats.signs[gi].clone()),
                Transform {
                    translation: local_arc(*off, R_OUT - 0.05, 1.7),
                    rotation: Quat::from_rotation_y(off.to_radians() + 180.0),
                    ..default()
                },
            ))
            .id();
        commands.entity(sec).add_child(win);
    }
    // ad posters between storefronts
    for (slot, off) in [(-9.0f32), 0.0, 9.0].iter().enumerate() {
        let gi = (i * 3 + slot) % 12;
        let ad = commands
            .spawn((
                Sec(i),
                Slot(slot),
                Mut::Ad,
                Mesh3d(plane(1.5, 2.1)),
                MeshMaterial3d(mats.ads[gi].clone()),
                Transform {
                    translation: local_arc(*off, R_OUT - 0.04, 3.3),
                    rotation: Quat::from_rotation_y(off.to_radians() + 180.0),
                    ..default()
                },
            ))
            .id();
        commands.entity(sec).add_child(ad);
    }
    // vending machines near outer wall
    for (slot, off) in [(-16.5f32), 16.5].iter().enumerate() {
        let mut e = commands.spawn((
            Sec(i),
            Slot(slot),
            Mut::Side,
            Prop {
                r: R_OUT - 0.75,
                off: *off,
                y: 1.0,
                rot: 180.0,
            },
            Obs(0.75),
        ));
        let body = commands
            .spawn((
                Sec(i),
                Mesh3d(box_m(1.1, 2.0, 0.7)),
                MeshMaterial3d(mats.ads[(i * 2 + slot) % 12].clone()),
            ))
            .id();
        commands.entity(e.id()).add_child(body);
        commands.entity(sec).add_child(e.id());
    }
    // potted plants + occasional bench
    for (slot, off) in [(-6.0f32), 6.0].iter().enumerate() {
        spawn_plant(commands, sec, mats, i, slot, *off, R_MID + 2.6);
    }
    let bench = commands
        .spawn((
            Sec(i),
            Slot(9),
            Mut::Side,
            Prop {
                r: R_IN + 1.2,
                off: 0.0,
                y: 0.0,
                rot: 0.0,
            },
            Obs(0.8),
        ))
        .id();
    let seat = commands
        .spawn((
            Sec(i),
            Mesh3d(box_m(1.8, 0.12, 0.55)),
            MeshMaterial3d(mats.bench[(i * 7 + 9) % 4].clone()),
            Transform::from_xyz(0.0, 0.45, 0.0),
        ))
        .id();
    commands.entity(bench).add_child(seat);
    commands.entity(sec).add_child(bench);

    // two mirrors on outer wall
    for (slot, off) in [(-10.5f32), 10.5].iter().enumerate() {
        spawn_mirror(commands, sec, mats, i, slot, *off);
    }
}

fn spawn_mirror(
    commands: &mut Commands,
    sec: Entity,
    mats: &Mats,
    i: usize,
    slot: usize,
    off: f32,
) {
    let mir = commands
        .spawn((
            Sec(i),
            Slot(slot),
            Mut::Ad,
            Mesh3d(plane(2.2, 2.8)),
            MeshMaterial3d(mats.mirror.clone()),
            Transform {
                translation: local_arc(off, R_OUT - 0.06, 1.75),
                rotation: Quat::from_rotation_y(off.to_radians() + 180.0),
                ..default()
            },
        ))
        .id();
    let nyan = commands
        .spawn((
            Sec(i),
            NyanQuad {
                phase: hash2(hash2(i as u64, slot as u64), 77) % 628 / 100.0,
            },
            Mesh3d(plane(2.0, 0.7)),
            MeshMaterial3d(mats.nyan.clone()),
            Transform::from_xyz(0.0, 0.55, 0.03),
        ))
        .id();
    commands.entity(mir).add_child(nyan);
    commands.entity(sec).add_child(mir);
}

fn spawn_plant(
    commands: &mut Commands,
    sec: Entity,
    mats: &Mats,
    i: usize,
    slot: usize,
    off: f32,
    r: f32,
) {
    let p = commands
        .spawn((
            Sec(i),
            Slot(slot),
            Mut::Side,
            Prop {
                r,
                off,
                y: 0.0,
                rot: 0.0,
            },
            Obs(0.55),
        ))
        .id();
    let pot = commands
        .spawn((
            Sec(i),
            Mesh3d(cyl(0.32, 0.5)),
            MeshMaterial3d(mats.pot.clone()),
            Transform::from_xyz(0.0, 0.25, 0.0),
        ))
        .id();
    let leaves = commands
        .spawn((
            Sec(i),
            Mesh3d(ball(0.6)),
            MeshMaterial3d(mats.plant.clone()),
            Transform::from_xyz(0.0, 1.15, 0.0),
        ))
        .id();
    commands.entity(p).add_children(&[pot, leaves]);
    commands.entity(sec).add_child(p);
}

fn spawn_food(
    commands: &mut Commands,
    sec: Entity,
    root: Entity,
    mats: &Mats,
    seed: u64,
    i: usize,
) {
    // 3 tables at deterministic slots
    let slots = [(-10.0f32, 41.6f32), (0.0, 44.4), (10.0, 41.6)];
    for (slot, (off, r)) in slots.iter().enumerate() {
        let t = commands
            .spawn((
                Sec(i),
                Slot(slot),
                Mut::Side,
                Prop {
                    r: *r,
                    off: *off,
                    y: 0.0,
                    rot: 0.0,
                },
                Obs(0.7),
            ))
            .id();
        let top = commands
            .spawn((
                Sec(i),
                Mesh3d(cyl(0.62, 0.06)),
                MeshMaterial3d(mats.marble.clone()),
                Transform::from_xyz(0.0, 0.72, 0.0),
            ))
            .id();
        let leg = commands
            .spawn((
                Sec(i),
                Mesh3d(cyl(0.08, 0.72)),
                MeshMaterial3d(mats.metal.clone()),
                Transform::from_xyz(0.0, 0.36, 0.0),
            ))
            .id();
        commands.entity(t).add_children(&[top, leg]);
        commands.entity(sec).add_child(t);
    }
    // benches inner rail
    for (slot, off) in [(-16.0f32), 16.0].iter().enumerate() {
        let b = commands
            .spawn((
                Sec(i),
                Slot(3 + slot),
                Mut::Side,
                Prop {
                    r: R_IN + 1.2,
                    off: *off,
                    y: 0.0,
                    rot: 0.0,
                },
                Obs(0.8),
            ))
            .id();
        let seat = commands
            .spawn((
                Sec(i),
                Mesh3d(box_m(1.8, 0.12, 0.55)),
                MeshMaterial3d(mats.bench[(i + slot) % 4].clone()),
                Transform::from_xyz(0.0, 0.45, 0.0),
            ))
            .id();
        commands.entity(b).add_child(seat);
        commands.entity(sec).add_child(b);
    }
    for (slot, off) in [(-4.0f32), 4.0].iter().enumerate() {
        spawn_plant(commands, sec, mats, i, 5 + slot, *off, R_MID + 2.4);
    }
    for (slot, off) in [(-12.0f32), 12.0].iter().enumerate() {
        spawn_mirror(commands, sec, mats, i, 7 + slot, *off);
    }
    // hotdog on the center table of section 6 (spawn hidden; revealed by hotdog_spawned)
    if i == 6 {
        let (off, r) = slots[1];
        let hd = commands
            .spawn((
                Sec(i),
                Hotdog,
                Prop {
                    r,
                    off,
                    y: 0.80,
                    rot: 0.0,
                },
                Visibility::Hidden,
            ))
            .id();
        let plate = commands
            .spawn((
                Sec(i),
                Mesh3d(cyl(0.24, 0.03)),
                MeshMaterial3d(mats.plate.clone()),
            ))
            .id();
        let bun = commands
            .spawn((
                Sec(i),
                Mesh3d(box_m(0.26, 0.09, 0.11)),
                MeshMaterial3d(mats.bun.clone()),
                Transform::from_xyz(0.0, 0.06, 0.0),
            ))
            .id();
        let sas = commands
            .spawn((
                Sec(i),
                Mesh3d(box_m(0.24, 0.07, 0.08)),
                MeshMaterial3d(mats.sausage.clone()),
                Transform::from_xyz(0.0, 0.11, 0.0),
            ))
            .id();
        let mus = commands
            .spawn((
                Sec(i),
                Mesh3d(box_m(0.24, 0.02, 0.03)),
                MeshMaterial3d(mats.mustard.clone()),
                Transform::from_xyz(0.0, 0.155, 0.0),
            ))
            .id();
        commands.entity(hd).add_children(&[plate, bun, sas, mus]);
        commands.entity(root).add_child(hd);
    }
    let _ = seed;
}

fn spawn_fountain(
    commands: &mut Commands,
    sec: Entity,
    root: Entity,
    mats: &Mats,
    seed: u64,
    i: usize,
) {
    let center = local_arc(0.0, MID, 0.0);
    let f = commands
        .spawn((
            Sec(i),
            Fountain { center },
            Mut::Water,
            Prop {
                r: MID,
                off: 0.0,
                y: 0.0,
                rot: 0.0,
            },
            Obs(2.0),
        ))
        .id();
    let rim = commands
        .spawn((
            Sec(i),
            Mesh3d(bevy::mesh::Torus::new(1.85, 0.25).into()),
            MeshMaterial3d(mats.marble.clone()),
            Transform::from_xyz(0.0, 0.55, 0.0),
        ))
        .id();
    let basin = commands
        .spawn((
            Sec(i),
            Mesh3d(cyl(1.85, 1.0)),
            MeshMaterial3d(mats.marble.clone()),
            Transform::from_xyz(0.0, 0.5, 0.0),
        ))
        .id();
    let water = commands
        .spawn((
            Sec(i),
            Mesh3d(bevy::mesh::Circle::new(1.75, 32).into()),
            MeshMaterial3d(mats.water.clone()),
            Transform {
                translation: Vec3::new(0.0, 0.85, 0.0),
                rotation: Quat::from_rotation_x(-90.0f32.to_radians()),
                ..default()
            },
        ))
        .id();
    let pillar = commands
        .spawn((
            Sec(i),
            Mesh3d(cyl(0.22, 1.3)),
            MeshMaterial3d(mats.marble.clone()),
            Transform::from_xyz(0.0, 1.2, 0.0),
        ))
        .id();
    let gold = commands
        .spawn((
            Sec(i),
            Mesh3d(ball(0.32)),
            MeshMaterial3d(mats.gold.clone()),
            Transform::from_xyz(0.0, 2.05, 0.0),
        ))
        .id();
    commands.entity(f).add_children(&[rim, basin, water, pillar, gold]);
    commands.entity(sec).add_child(f);

    // penny on the center table of section 2's food court, or a rim ledge
    if i == 0 {
        // small pedestal with penny near rim
        let ped = commands
            .spawn((
                Sec(i),
                Prop {
                    r: MID - 2.7,
                    off: 0.0,
                    y: 0.0,
                    rot: 0.0,
                },
                Obs(0.45),
            ))
            .id();
        let post = commands
            .spawn((
                Sec(i),
                Mesh3d(cyl(0.14, 1.0)),
                MeshMaterial3d(mats.marble.clone()),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ))
            .id();
        commands.entity(ped).add_child(post);
        commands.entity(sec).add_child(ped);
        let penny = commands
            .spawn((
                Sec(i),
                Penny,
                Prop {
                    r: MID - 2.7,
                    off: 0.0,
                    y: 1.05,
                    rot: 0.0,
                },
            ))
            .id();
        let coin = commands
            .spawn((
                Sec(i),
                Mesh3d(cyl(0.14, 0.03)),
                MeshMaterial3d(mats.gold.clone()),
            ))
            .id();
        commands.entity(penny).add_child(coin);
        commands.entity(root).add_child(penny);
    }
    let _ = seed;
}

fn spawn_atrium(
    commands: &mut Commands,
    sec: Entity,
    mats: &Mats,
    seed: u64,
    i: usize,
) {
    // floating shapes beyond the glass inner wall
    for s in 0..6u64 {
        let h = hash2(seed, i as u64 * 100 + s);
        let r = 24.0 + (h % 1000) as f32 / 1000.0 * 10.0;
        let off = -15.0 + (h % 3000) as f32 / 3000.0 * 30.0;
        let y = 2.0 + ((h >> 13) % 1000) as f32 / 1000.0 * 6.0;
        let mesh = match (h >> 29) % 3 {
            0 => ball(1.6),
            1 => box_m(2.4, 2.4, 2.4),
            _ => bevy::mesh::Torus::new(1.5, 0.5).into(),
        };
        commands.spawn((
            Sec(i),
            FloatShape {
                speed: 0.1 + ((h >> 40) % 40) as f32 / 100.0,
                axis: Vec3::new(
                    ((h >> 20) % 100) as f32 / 100.0 - 0.5,
                    1.0,
                    ((h >> 30) % 100) as f32 / 100.0 - 0.5,
                )
                .normalize(),
            },
            Mesh3d(mesh),
            MeshMaterial3d(mats.ads[(i + s as usize) % 12].clone()),
            Transform::from_translation(local_arc(off, r, y)),
        ));
    }
    // a couple of benches and a mirror for life
    let b = commands
        .spawn((
            Sec(i),
            Slot(0),
            Mut::Side,
            Prop {
                r: R_IN + 1.2,
                off: 8.0,
                y: 0.0,
                rot: 0.0,
            },
            Obs(0.8),
        ))
        .id();
    let seat = commands
        .spawn((
            Sec(i),
            Mesh3d(box_m(1.8, 0.12, 0.55)),
            MeshMaterial3d(mats.bench[i % 4].clone()),
            Transform::from_xyz(0.0, 0.45, 0.0),
        ))
        .id();
    commands.entity(b).add_child(seat);
    commands.entity(sec).add_child(b);
    spawn_mirror(commands, sec, mats, i, 1, 6.0);
}

pub fn build_world(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    smats: &mut ResMut<Assets<StandardMaterial>>,
    mats: &Mats,
    mut obstacles: ResMut<Obstacles>,
    mut wm: ResMut<WorldMats>,
    seed: u64,
) {
    obstacles.clear();
    let root = commands
        .spawn((WorldRoot, Name::new("world")))
        .id();

    // per-section wall material variants
    wm.wall_variants.clear();
    wm.cur_variant.clear();
    for i in 0..N_SEC {
        let mut vars = [Handle::default(), Handle::default(), Handle::default()];
        for v in 0..3 {
            let d = texturegen::wall_tile(seed, v as f32 * 0.08);
            vars[v] = mat_tex(images, smats, 256, 256, d, Color::WHITE, 0.0);
        }
        wm.wall_variants.push(vars);
        wm.cur_variant
            .push((hash2(seed, i as u64) % 3) as usize);
    }

    for i in 0..N_SEC {
        spawn_section_shell(commands, root, mats, &wm, seed, i);
    }

    // penny on the food court center table (section 2) - spawn visible
    // (a second decorative pedestal exists at fountain 0; the real pickup is here)
    rebuild_obstacles(commands);
}

/// deterministic hotdog reveal + obstacle rebuild after any mutation
pub fn rebuild_obstacles(mut commands: Commands) {
    // implemented as system via ObstacleBuild request
    commands.insert_resource(ObstacleBuildTick);
}

/// request resource so the system owning Obstacles can rebuild from Prop entities
#[derive(Resource)]
pub struct ObstacleBuildTick;
