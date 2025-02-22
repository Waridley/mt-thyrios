#![feature(iter_array_chunks)]
#![cfg_attr(
	feature = "dev_tools",
	feature(path_add_extension, generic_const_exprs)
)]

use crate::util::gltf::{GlbSaver, GltfZUpTransformer};
use crate::util::{FinishedProcessing, make_new_gltf_scenes_z_up};
use bevy::asset::processor::LoadTransformAndSave;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::gltf::{GltfLoader, GltfPlugin};
use bevy::log::LogPlugin;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::window::{PresentMode, PrimaryWindow};
use std::path::PathBuf;

#[cfg(feature = "dev_tools")]
pub mod dev_tools;
pub mod game;
pub mod ld;
pub mod main_menu;
pub mod settings_menu;
pub mod setup_tracking;
#[cfg(not(feature = "skip_splash"))]
pub mod splash;
pub mod state;
pub mod steam;
pub mod ui;
pub mod util;

fn main() -> AppExit {
	let mut app = App::new();
	let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").map(PathBuf::from);
	// `assets` folder is next to cargo project during dev, but next to binary in releases.
	let file_path = cargo_dir
		.as_ref()
		.map(|dir| {
			dir.parent()
				.unwrap()
				.join("assets")
				.to_string_lossy()
				.into_owned()
		})
		.unwrap_or("assets".into());
	let processed_file_path = cargo_dir
		.as_ref()
		.map(|dir| {
			dir.parent()
				.unwrap()
				.join("imported_assets/Default")
				.to_string_lossy()
				.into_owned()
		})
		.unwrap_or("imported_assets".into());

	let mut def_plugs = DefaultPlugins
		.set(
			GltfPlugin::default()
				.add_custom_vertex_attribute(
					"TERRAIN_WEIGHTS_0_3",
					game::mtn::terrain::ATTRIBUTE_TERRAIN_WEIGHTS_0_3,
				)
				.add_custom_vertex_attribute(
					"TERRAIN_WEIGHTS_4_7",
					game::mtn::terrain::ATTRIBUTE_TERRAIN_WEIGHTS_4_7,
				),
		)
		.set(AssetPlugin {
			file_path,
			processed_file_path,
			mode: AssetMode::Processed,
			..default()
		});

	#[cfg(feature = "dev_tools")]
	let def_plugs = def_plugs.set(LogPlugin {
		custom_layer: bevy_console::make_layer,
		..default()
	});

	app.add_plugins(def_plugs);
	#[cfg(feature = "dev_tools")]
	app.add_plugins((FrameTimeDiagnosticsPlugin, WireframePlugin));
	app.add_plugins(TemporalAntiAliasPlugin);

	app.add_plugins((
		#[cfg(not(feature = "skip_splash"))]
		splash::SplashPlugin,
		main_menu::MainMenuPlugin,
		settings_menu::SettingsMenuPlugin,
		game::GamePlugin,
		ld::LoadingScreenPlugin,
		state::GlobalStatePlugin,
		steam::SteamIntegration,
		ui::UiPlugin,
		#[cfg(feature = "dev_tools")]
		dev_tools::ToolsPlugin,
	))
	.add_systems(Startup, setup)
	.add_event::<FinishedProcessing<Gltf>>()
	// .add_systems(First, make_new_gltf_scenes_z_up)
	.register_asset_processor(LoadTransformAndSave::<
		GltfLoader,
		GltfZUpTransformer,
		GlbSaver,
	>::new(GltfZUpTransformer, GlbSaver));

	app.run()
}

pub fn setup(mut window: Single<&mut Window, With<PrimaryWindow>>) {
	window.present_mode = PresentMode::AutoNoVsync;
}
