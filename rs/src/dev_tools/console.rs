use crate::dev_tools::setup_graph_vis::SetupGraphVisState;
use crate::game::GameSetupKey;
use crate::game::ocean::{OceanSurface, StormIntensity};
use crate::state::GlobalState;
use crate::ui::egui::Align2;
use bevy::prelude::*;
use bevy_console::{
	AddConsoleCommand, BevyLogBuffer, ConsoleCommand, ConsoleConfiguration, ConsolePlugin, reply,
	reply_failed,
};
use clap::builder::{PossibleValue, TypedValueParser};
use clap::{Arg, Error, Parser, Subcommand};
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::num::ParseFloatError;

pub struct DevConsolePlugin;

impl Plugin for DevConsolePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(ConsoleConfiguration {
			anchor: Some((Align2::CENTER_TOP, [0.0, 0.0].into())),
			font_size: 11.0,
			..default()
		})
		.add_plugins(ConsolePlugin)
		.add_console_command::<About, _>(about_game)
		.add_console_command::<SetNextState, _>(set_next_state)
		.add_console_command::<ViewSetupGraph, _>(view_setup_graph)
		.add_console_command::<SetLight, _>(set_light)
		.add_console_command::<AdjustOcean, _>(adjust_ocean);
	}
}

/// Game version information
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "about")]
pub struct About {}

#[rustfmt::skip]
pub fn about_game(mut cmd: ConsoleCommand<About>) {
	if let Some(Ok(_)) = cmd.take() {
		cmd.reply(format!("
{} {}
{}

By {}
",
			clap::crate_name!(), clap::crate_version!(),
			clap::crate_description!(),
			clap::crate_authors!())
		)
	}
}

/// Enter a specific State
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

/// Visualize game setup dependencies
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "setup_graph")]
pub struct ViewSetupGraph {}

pub fn view_setup_graph(mut cmd: ConsoleCommand<ViewSetupGraph>, mut cmds: Commands) {
	if let Some(Ok(_)) = cmd.take() {
		cmds.init_resource::<SetupGraphVisState<GameSetupKey>>();
	}
}

/// Adjust light properties
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "light")]
#[clap(infer_subcommands = true)]
pub struct SetLight {
	#[command(subcommand)]
	action: GetSetLightValue,
}

pub fn set_light(
	mut command: ConsoleCommand<SetLight>,
	mut q: Option<Single<&mut SpotLight, With<crate::game::cam::CamLight>>>,
) {
	if let Some(Ok(cmd)) = command.take() {
		let Some(mut q) = q else {
			command.reply_failed("No light spawned");
			return;
		};
		match cmd.action {
			GetSetLightValue::Intensity { value } => {
				if let Some(intensity) = value {
					q.intensity = intensity
				} else {
					command.reply(format!("{:e}", q.intensity))
				}
			}
			GetSetLightValue::OuterAngle { value } => {
				if let Some(angle) = value {
					q.outer_angle = angle.to_radians()
				} else {
					command.reply(format!("{}", q.outer_angle.to_degrees()))
				}
			}
			GetSetLightValue::InnerAngle { value } => {
				if let Some(angle) = value {
					q.inner_angle = angle.to_radians()
				} else {
					command.reply(format!("{}", q.inner_angle.to_degrees()))
				}
			}
		}
	}
}

#[derive(Subcommand, Debug)]
enum GetSetLightValue {
	#[command(aliases = &["brightness", "lumens", "luminosity"])]
	Intensity {
		value: Option<f32>,
	},
	OuterAngle {
		value: Option<f32>,
	},
	InnerAngle {
		value: Option<f32>,
	},
}

/// Adjust ocean and storm parameters
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "ocean")]
#[clap(infer_subcommands = true)]
pub struct AdjustOcean {
	#[command(subcommand)]
	action: GetSetOceanValue,
}

#[derive(Subcommand, Debug)]
pub enum GetSetOceanValue {
	#[command(alias = "intensity")]
	StormIntensity {
		#[arg(allow_hyphen_values = true)]
		value: Option<f32>,
	},
	#[command(alias = "z")]
	Height {
		#[arg(allow_hyphen_values = true)]
		value: Option<f32>,
	},
}

pub fn adjust_ocean(
	mut command: ConsoleCommand<AdjustOcean>,
	mut ocean: Option<Single<&mut Transform, With<OceanSurface>>>,
	mut storm: Option<ResMut<StormIntensity>>,
) {
	if let Some(Ok(cmd)) = command.take() {
		match cmd.action {
			GetSetOceanValue::StormIntensity { value } => {
				let Some(storm) = storm.as_mut() else {
					reply_failed!(command, "Storm resource does not exist");
					return;
				};
				if let Some(intensity) = value {
					storm.0 = intensity;
				}
				reply!(command, "{:?}", &**storm);
			}
			GetSetOceanValue::Height { value } => {
				let Some(mut ocean) = ocean else {
					reply_failed!(command, "OceanSurface is not spawned");
					return;
				};
				if let Some(z) = value {
					ocean.translation.z = z;
				}
				reply!(command, "Ocean height: {}", ocean.translation.z);
			}
		}
	}
}
