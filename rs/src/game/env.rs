use crate::state::GlobalState::{self, InGame};
use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_environment);
	}
}

pub fn setup_environment(
	mut cmds: Commands,
) {
	// cmds.insert_resource(AmbientLight {
	// 	brightness: 40.0,
	// 	..default()
	// });
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct Sunlight;
