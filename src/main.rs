fn main() {
    let mut app = bevy::prelude::App::new();
    dreammall::app::build(&mut app);
    app.run();
}
