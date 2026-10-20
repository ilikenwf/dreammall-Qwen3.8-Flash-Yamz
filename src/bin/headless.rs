//! Headless integration driver: boots the real game under a display server
//! (xvfb) and drives a scripted end-to-end check of the win flows.
//!
//! Run 1 (regression): clean start state, penny hidden until its seeded reveal
//! pass, mutation reveals penny+hotdog, atrium glass is never mutated opaque,
//! pressing E at a fountain without the penny does nothing.
//!
//! Run 2 (water win): grant the penny, verify the THROW prompt near a
//! fountain, click-grab the cursor, throw -> Water win -> fade -> ending
//! screen with cursor released and both buttons present.
//!
//! Input timing: manual ButtonInput presses are consumed by the game one
//! frame later (InputPlugin recomputes just_pressed in PreUpdate), so every
//! act/verify pair has a wait phase between it.
//!
//! Exits 0 on success, 1 on the first failed assertion.

use bevy::input::InputSystems;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use dreammall::game::{hash2, EYE_H};
use dreammall::shared::*;

thread_local! {
    static S: std::cell::RefCell<St> = std::cell::RefCell::new(St::default());
}

#[derive(Default)]
struct St {
    n: u32,
    phase: u32,
    settle: u32,
    glass: Option<(Entity, Handle<StandardMaterial>)>,
    reveal_pass: u32,
}

/// exit without running atexit handlers: on headless Linux the ALSA/Mesa
/// teardown after process::exit segfaults; results are already flushed
fn hard_exit(code: i32) -> ! {
    #[cfg(unix)]
    unsafe {
        libc::_exit(code)
    }
    #[cfg(not(unix))]
    std::process::exit(code)
}

fn fail(msg: &str) -> ! {
    println!("FAIL: {msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    hard_exit(1)
}

fn pass(name: &str) {
    println!("PASS: {name}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

fn any_text_contains(world: &mut World, needle: &str) -> bool {
    let mut q = world.query::<&Text>();
    q.iter(world).any(|t| t.0.contains(needle))
}

fn ui_text(world: &mut World, node: &str) -> Option<String> {
    let mut q = world.query::<(&Name, &Children)>();
    for (name, kids) in q.iter(world) {
        if name.as_str() == node {
            for k in kids.iter() {
                if let Some(t) = world.get::<Text>(k) {
                    return Some(t.0.clone());
                }
            }
        }
    }
    None
}

fn fountain_center(world: &mut World) -> Vec3 {
    let mut q = world.query::<&Fountain>();
    match q.iter(world).next().map(|f| f.center) {
        Some(c) => c,
        None => fail("no fountain entity"),
    }
}

/// move the player onto `pos` and resync theta_prev so the teleport cannot
/// register as lap progress
fn teleport(world: &mut World, pos: Vec3) {
    let theta = pos.x.atan2(pos.z).to_degrees();
    {
        let mut q = world.query_filtered::<&mut Transform, With<Player>>();
        match q.iter_mut(world).next() {
            Some(mut tf) => tf.translation = pos,
            None => fail("no player entity"),
        }
    }
    let mut run = world.resource_mut::<Run>();
    run.theta_prev = theta;
    run.idle = 0.0;
}

fn driver(world: &mut World) {
    // wait until build_pending has actually spawned the world
    if !world.contains_resource::<Run>() {
        return;
    }
    {
        let mut fq = world.query::<&Fountain>();
        if fq.iter(world).next().is_none() {
            return;
        }
        let mut pq = world.query_filtered::<Entity, With<Player>>();
        if pq.iter(world).next().is_none() {
            return;
        }
    }
    let n = S.with(|s| {
        let mut s = s.borrow_mut();
        s.n += 1;
        s.n
    });
    if n > 2400 {
        fail("frame budget exhausted");
    }

    let phase = S.with(|s| s.borrow().phase);
    if phase >= 20 {
        flight(world, n, phase);
        return;
    }
    match phase {
        // ---------- FLIGHT MODE: separate scripted run ----------
        0 if std::env::var("DREAMMALL_FLIGHT").is_ok() => {
            // drive the seeded mutation directly so the hotdog reveals
            {
                let mut run = world.resource_mut::<Run>();
                run.pass = 1;
                run.mutate_flag = true;
            }
            S.with(|s| s.borrow_mut().phase = 20);
        }
        // ---------- RUN 1: clean start ----------
        0 => {
            {
                let run = world.resource::<Run>();
                if run.has_penny || run.penny_spawned || run.has_hotdog || run.hotdog_spawned {
                    fail("start state not clean");
                }
                if run.won.is_some() || run.prompt.is_some() || run.mode != Mode::Walking {
                    fail("start mode/win/prompt not clean");
                }
                let seed = run.seed;
                S.with(|s| s.borrow_mut().reveal_pass = 1 + (hash2(seed, 101) % 2) as u32);
            }
            pass("clean start");

            // pennies exist but are hidden
            let mut q = world.query_filtered::<&Visibility, With<Penny>>();
            let vis: Vec<Visibility> = q.iter(world).copied().collect();
            if vis.is_empty() || !vis.iter().all(|v| *v == Visibility::Hidden) {
                fail("pennies not all hidden at start");
            }
            pass("penny entities hidden at start");

            // atrium glass uses a Blend material; capture handle for later
            let mut q = world.query::<(Entity, &Role, &MeshMaterial3d<StandardMaterial>)>();
            let smats = world.resource::<Assets<StandardMaterial>>();
            let mut found = 0usize;
            let mut cap: Option<(Entity, Handle<StandardMaterial>)> = None;
            for (e, role, mm) in q.iter(world) {
                if matches!(role, Role::Glass) {
                    found += 1;
                    match smats.get(&mm.0) {
                        Some(m) if m.alpha_mode == AlphaMode::Blend => {
                            if cap.is_none() {
                                cap = Some((e, mm.0.clone()));
                            }
                        }
                        _ => fail("glass material is not Blend"),
                    }
                }
            }
            if found == 0 {
                fail("no Role::Glass entities");
            }
            S.with(|s| s.borrow_mut().glass = cap);
            pass("atrium glass uses Blend material");

            // walking hint visible
            match ui_text(world, "hint") {
                Some(t) if t.contains("W forward") => pass("walking hint present"),
                other => fail(&format!("hint text wrong: {other:?}")),
            }

            // regression: stand at a fountain WITHOUT the penny
            let c = fountain_center(world);
            teleport(world, Vec3::new(c.x, EYE_H, c.z));
            S.with(|s| s.borrow_mut().phase = 1);
        }
        1 => {
            world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
            S.with(|s| s.borrow_mut().phase = 2);
        }
        2 => {
            // one frame: interactions consumed the E press last Update
            S.with(|s| s.borrow_mut().phase = 3);
        }
        3 => {
            {
                let run = world.resource::<Run>();
                if run.won.is_some() {
                    fail("won without penny!");
                }
                if run.prompt.is_some() {
                    fail("prompt appeared without penny");
                }
            }
            world.resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyE);
            pass("no win without penny (E at fountain)");

            // queue the seeded reveal pass directly
            let rp = S.with(|s| s.borrow().reveal_pass);
            {
                let mut run = world.resource_mut::<Run>();
                run.pass = rp;
                run.mutate_flag = true;
            }
            println!("INFO: queued mutation at pass {rp}");
            S.with(|s| s.borrow_mut().phase = 4);
        }
        // ---------- RUN 1: mutation reveal ----------
        4 => {
            {
                let run = world.resource::<Run>();
                if !run.penny_spawned {
                    if n > 150 {
                        fail("penny never revealed after mutation");
                    }
                    return;
                }
                if !run.hotdog_spawned {
                    fail("hotdog not revealed at pass>=1");
                }
            }
            let mut q = world.query_filtered::<&Visibility, With<Penny>>();
            if !q.iter(world).all(|v| *v == Visibility::Visible) {
                fail("pennies not Visible after reveal");
            }
            pass("penny revealed (Visible)");
            pass("hotdog revealed at pass>=1");

            // glass untouched: same handle, still Blend
            let (ge, gh) = S.with(|s| s.borrow().glass.clone().unwrap());
            let still = world
                .get::<MeshMaterial3d<StandardMaterial>>(ge)
                .map(|mm| mm.0.clone());
            match still {
                Some(h) if h == gh => {}
                _ => fail("glass entity material handle changed by mutation"),
            }
            match world.resource::<Assets<StandardMaterial>>().get(&gh) {
                Some(m) if m.alpha_mode == AlphaMode::Blend => {}
                _ => fail("glass material no longer Blend after mutation"),
            }
            pass("glass untouched by mutation");

            match &world.resource::<Run>().toast {
                Some((m, _)) if m == "SOMETHING GLINTS ON A TABLE SOMEWHERE." => {
                    pass("penny reveal toast")
                }
                other => fail(&format!("reveal toast wrong: {other:?}")),
            }
            pass("RUN1 OK");

            // ---------- RUN 2: water win ----------
            world.resource_mut::<Run>().has_penny = true;
            let c = fountain_center(world);
            teleport(world, Vec3::new(c.x, EYE_H, c.z));
            S.with(|s| s.borrow_mut().phase = 5);
        }
        // ---------- RUN 2: prompt ----------
        5 => {
            match &world.resource::<Run>().prompt {
                Some(p) if p.contains("THROW THE PENNY") => pass("throw prompt near fountain"),
                None => {
                    if n > 250 {
                        fail("throw prompt never appeared");
                    }
                    return;
                }
                Some(p) => fail(&format!("unexpected prompt: {p}")),
            }
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            S.with(|s| s.borrow_mut().phase = 6);
        }
        6 => {
            // one frame: cursor_ctrl grabbed last Update
            S.with(|s| s.borrow_mut().phase = 7);
        }
        // ---------- RUN 2: cursor grab ----------
        7 => {
            if !world.resource::<Run>().locked {
                if n > 300 {
                    fail("cursor never grabbed after click");
                }
                return;
            }
            let mut q = world.query_filtered::<&CursorOptions, With<PrimaryWindow>>();
            match q.iter(world).next() {
                Some(co) if co.grab_mode == CursorGrabMode::Locked => {
                    pass("cursor grabbed on click")
                }
                other => fail(&format!("cursor grab_mode wrong: {other:?}")),
            }
            world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
            S.with(|s| s.borrow_mut().phase = 8);
        }
        8 => {
            // one frame: interactions triggered the win last Update
            S.with(|s| s.borrow_mut().phase = 9);
        }
        // ---------- RUN 2: win ----------
        9 => {
            match world.resource::<Run>().won {
                Some(WinKind::Water) => pass("penny throw wins (Water)"),
                Some(w) => fail(&format!("wrong win kind: {w:?}")),
                None => {
                    if n > 500 {
                        fail("water win never triggered");
                    }
                    return;
                }
            }
            world.resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyE);
            world
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            S.with(|s| s.borrow_mut().phase = 10);
        }
        10 => {
            if *world.resource::<State<GS>>().get() != GS::Ended {
                if n > 1500 {
                    fail("ending state never entered");
                }
                return;
            }
            pass("ending state after fade");
            // let OnEnter(Ended) systems run + land
            S.with(|s| s.borrow_mut().settle = n + 4);
            S.with(|s| s.borrow_mut().phase = 11);
        }
        11 => {
            if n < S.with(|s| s.borrow().settle) {
                return;
            }
            let mut q = world.query_filtered::<&CursorOptions, With<PrimaryWindow>>();
            match q.iter(world).next() {
                Some(co) if co.grab_mode == CursorGrabMode::None => {}
                other => fail(&format!("cursor still grabbed at ending: {other:?}")),
            }
            if world.resource::<Run>().locked {
                fail("run.locked still true at ending");
            }
            pass("cursor released on win");

            {
                let mut q = world.query_filtered::<Entity, With<SleepBtn>>();
                if q.iter(world).next().is_none() {
                    fail("no SLEEP button on ending screen");
                }
                let mut q = world.query_filtered::<Entity, With<ExitBtn>>();
                if q.iter(world).next().is_none() {
                    fail("no EXIT button on ending screen");
                }
            }
            pass("ending buttons exist");

            for node in ["hint", "prompt", "toast"] {
                match ui_text(world, node) {
                    Some(t) if t.is_empty() => {}
                    other => fail(&format!("{node} text not cleared at ending: {other:?}")),
                }
            }
            pass("hud cleared on ending");

            pass("RUN2 OK");
            println!("ALL CHECKS PASSED");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            hard_exit(0);
        }
        _ => {}
    }
}

/// scripted flight-win run (DREAMMALL_FLIGHT=1): hotdog pickup -> eat ->
/// flight -> fountain dive -> Flight win -> ending screen
fn flight(world: &mut World, n: u32, phase: u32) {
    match phase {
        20 => {
            if !world.resource::<Run>().hotdog_spawned {
                if n > 150 {
                    fail("hotdog never revealed");
                }
                return;
            }
            pass("hotdog revealed (flight run)");
            let mut q = world.query_filtered::<&GlobalTransform, With<Hotdog>>();
            let hd = match q.iter(world).next() {
                Some(t) => t.translation(),
                None => fail("no hotdog entity"),
            };
            teleport(world, Vec3::new(hd.x + 1.0, EYE_H, hd.z));
            S.with(|s| s.borrow_mut().phase = 21);
        }
        21 => {
            match &world.resource::<Run>().prompt {
                Some(p) if p.contains("TAKE THE HOTDOG") => pass("hotdog take prompt"),
                None => {
                    if n > 250 {
                        fail("hotdog prompt never appeared");
                    }
                    return;
                }
                Some(p) => fail(&format!("unexpected prompt near hotdog: {p}")),
            }
            world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
            S.with(|s| s.borrow_mut().phase = 22);
        }
        22 => {
            {
                let run = world.resource::<Run>();
                if !run.has_hotdog {
                    if n > 300 {
                        fail("hotdog pickup never happened");
                    }
                    return;
                }
                if !run.prompt.as_deref().unwrap_or("").contains("EAT THE HOTDOG") {
                    if n > 300 {
                        fail("eat prompt never appeared");
                    }
                    return;
                }
            }
            pass("hotdog picked up");
            world.resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyE);
            S.with(|s| s.borrow_mut().phase = 23);
        }
        23 => {
            world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
            S.with(|s| s.borrow_mut().phase = 24);
        }
        24 => {
            {
                let run = world.resource::<Run>();
                if run.mode != Mode::Flying {
                    if n > 300 {
                        fail("eating hotdog never entered flight");
                    }
                    return;
                }
            }
            pass("hotdog eat enters flight");
            world.resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyE);
            // rise win: past the glass roof (y > 55). Fountain dives are the
            // Water kind by design (water ending); covered by the water run.
            teleport(world, Vec3::new(0.0, 55.5, 0.0));
            S.with(|s| s.borrow_mut().phase = 25);
        }
        25 => {
            match world.resource::<Run>().won {
                Some(WinKind::Flight) => pass("rise past glass roof wins (Flight)"),
                Some(w) => fail(&format!("wrong win kind: {w:?}")),
                None => {
                    if n > 600 {
                        fail("flight win never triggered");
                    }
                    return;
                }
            }
            S.with(|s| s.borrow_mut().phase = 26);
        }
        26 => {
            if *world.resource::<State<GS>>().get() != GS::Ended {
                if n > 1800 {
                    fail("ending state never entered (flight)");
                }
                return;
            }
            S.with(|s| s.borrow_mut().settle = n + 4);
            S.with(|s| s.borrow_mut().phase = 27);
        }
        27 => {
            if n < S.with(|s| s.borrow().settle) {
                return;
            }
            if !any_text_contains(world, "RISE THROUGH THE GLASS ROOF") {
                fail("flight ending message missing");
            }
            pass("flight ending message");
            {
                let mut q = world.query_filtered::<Entity, With<SleepBtn>>();
                if q.iter(world).next().is_none() {
                    fail("no SLEEP button (flight ending)");
                }
            }
            pass("FLIGHT RUN OK");
            println!("ALL CHECKS PASSED");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            hard_exit(0);
        }
        _ => {}
    }
}

fn main() {
    let mut app = App::new();
    dreammall::app::build(&mut app);
    // after input so manual presses land next frame, deterministic ordering
    app.add_systems(PreUpdate, driver.after(InputSystems));
    app.run();
}
