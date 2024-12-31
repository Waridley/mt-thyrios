use bevy::prelude::*;
use crate::state::GlobalState;

pub mod cam;
pub mod env;
pub mod input;
pub mod mtn;
pub mod ocean;

pub struct GamePlugin;

impl Plugin for GamePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			cam::GameCamPlugin,
			env::EnvironmentPlugin,
			input::InputPlugin,
			mtn::MountainPlugin,
			ocean::OceanPlugin,
		))
			.add_systems(OnEnter(GlobalState::InGame), setup_game);
	}
}

pub fn setup_game(mut commands: Commands) {
	info!("Setting up game");
}
