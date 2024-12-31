use bevy::prelude::*;
use crate::state::GlobalState;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(GlobalState::MainMenu), setup_main_menu);
	}
}

fn setup_main_menu(mut commands: Commands, mut next_state: ResMut<NextState<GlobalState>>) {
	warn!("TODO: set up main menu");
	next_state.set(GlobalState::InGame);
}
