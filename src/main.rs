use bevy::prelude::*;

use gamejam_2023::game::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_plugins(GamePlugins);
    app.run();
}
