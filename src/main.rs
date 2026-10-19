mod app;
mod shared;
mod world;

fn main() {
    let mut app = bevy::prelude::App::new();
    app::build(&mut app);
    app.run();
}
