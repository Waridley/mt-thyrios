use std::path::PathBuf;
use bevy::prelude::*;

pub mod game;
pub mod main_menu;
#[cfg(not(feature = "skip_splash"))]
pub mod splash;
pub mod state;
pub mod steam;

fn main() -> AppExit {
	let mut app = App::new();
	
	// `assets` folder is next to cargo project during dev, but next to binary in releases.
	let file_path = std::env::var("CARGO_MANIFEST_DIR")
		.map(|dir| {
			let mut path = PathBuf::from(dir);
			path.pop();
			path.push("assets");
			path.to_string_lossy().into_owned()
		})
		.unwrap_or("assets".into());
	
	app.add_plugins(DefaultPlugins
		.set(AssetPlugin {
			file_path,
			..default()
		}))
		.add_plugins((
			state::GlobalStatePlugin,
			steam::SteamIntegration,
			#[cfg(not(feature = "skip_splash"))]
			splash::SplashPlugin,
			main_menu::MainMenuPlugin,
			game::GamePlugin,
		))
		.add_systems(Startup, setup);
	
	app.run()
}

fn setup(mut cmds: Commands) {

}
