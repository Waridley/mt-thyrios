use crate::dev_tools::setup_graph_vis::SetupGraphVisState;
use crate::game::GameSetupKey;
use crate::state::GlobalState;
use bevy::prelude::*;
use bevy_console::{AddConsoleCommand, BevyLogBuffer, ConsoleCommand, ConsolePlugin};
use clap::Parser;

pub struct DevConsolePlugin;

impl Plugin for DevConsolePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ConsolePlugin)
			.add_console_command::<About, _>(about_game)
			.add_console_command::<SetNextState, _>(set_next_state)
			.add_console_command::<ViewSetupGraph, _>(view_setup_graph);
	}
}

#[derive(Parser, Debug, ConsoleCommand)]
#[clap(about, version)]
#[command(name = "about")]
pub struct About {}

pub fn about_game(mut cmd: ConsoleCommand<About>) {
	if let Some(Ok(cmd)) = cmd.take() {
		info!("{cmd:?}")
	}
}

#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "goto")]
pub struct SetNextState {
	state: GlobalState,
}

pub fn set_next_state(
	mut cmd: ConsoleCommand<SetNextState>,
	mut next_state: ResMut<NextState<GlobalState>>,
) {
	if let Some(Ok(cmd)) = cmd.take() {
		next_state.set(cmd.state);
	}
}

#[derive(Parser, Debug, ConsoleCommand)]
#[clap(about)]
#[command(name = "setup_graph")]
pub struct ViewSetupGraph {}

pub fn view_setup_graph(mut cmd: ConsoleCommand<ViewSetupGraph>, mut cmds: Commands) {
	if let Some(Ok(_)) = cmd.take() {
		cmds.init_resource::<SetupGraphVisState<GameSetupKey>>();
	}
}
