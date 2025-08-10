use bevy::prelude::*;

pub struct GlobalStatePlugin;

impl Plugin for GlobalStatePlugin {
	fn build(&self, app: &mut App) {
		app.init_state::<GlobalState>()
			.enable_state_scoped_entities::<GlobalState>();
	}
}

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(
	feature = "dev_tools",
	derive(clap::ValueEnum),
	clap(rename_all = "verbatim")
)]
pub enum GlobalState {
	#[cfg_attr(not(feature = "skip_splash"), default)]
	Splash,
	#[cfg_attr(feature = "skip_splash", default)]
	MainMenu,
	LoadingGame,
	#[cfg_attr(feature = "dev_tools", value(skip))] // Prevent entering InGame without loading
	InGame,
}
