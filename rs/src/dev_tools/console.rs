use crate::dev_tools::setup_graph_vis::SetupGraphVisState;
use crate::game::ocean::{OceanMaterial, OceanSurface, Storm};
use crate::game::GameSetupKey;
use crate::state::GlobalState;
use crate::ui::egui::Align2;
use bevy::{pbr::ExtendedMaterial, prelude::*};
use bevy_console::{
	reply, reply_failed, AddConsoleCommand, ConsoleCommand, ConsoleConfiguration,
	ConsolePlugin,
};
use clap::builder::PossibleValue;
use clap::{Parser, Subcommand};

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
		.add_console_command::<AdjustOcean, _>(adjust_ocean)
		.add_console_command::<AdjustStorm, _>(adjust_storm)
		.add_console_command::<Hide, _>(set_visibility::<Hide>)
		.add_console_command::<Show, _>(set_visibility::<Show>);
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
	q: Option<Single<&mut SpotLight, With<crate::game::cam::CamLight>>>,
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
				}
				reply!(command, "{:e}", q.intensity);
			}
			GetSetLightValue::OuterAngle { value } => {
				if let Some(angle) = value {
					q.outer_angle = angle.to_radians()
				}
				reply!(command, "{}", q.outer_angle.to_degrees());
			}
			GetSetLightValue::InnerAngle { value } => {
				if let Some(angle) = value {
					q.inner_angle = angle.to_radians()
				}
				reply!(command, "{}", q.inner_angle.to_degrees());
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

/// Adjust ocean parameters
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "ocean")]
#[clap(infer_subcommands = true)]
pub struct AdjustOcean {
	#[command(subcommand)]
	action: GetSetOceanValue,
}

#[derive(Subcommand, Debug)]
pub enum GetSetOceanValue {
	#[command(alias = "z")]
	Height {
		#[arg(allow_hyphen_values = true)]
		value: Option<f32>,
	},
	Seed {
		value: Option<String>,
	},
}

pub fn adjust_ocean(
	mut command: ConsoleCommand<AdjustOcean>,
	mut ocean: Option<
		Single<
			(
				&mut Transform,
				&MeshMaterial3d<ExtendedMaterial<StandardMaterial, OceanMaterial>>,
			),
			With<OceanSurface>,
		>,
	>,
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
) {
	if let Some(Ok(cmd)) = command.take() {
		let Some((xform, mat)) = ocean.as_deref_mut() else {
			reply_failed!(command, "OceanSurface is not spawned");
			return;
		};
		match cmd.action {
			GetSetOceanValue::Height { value } => {
				if let Some(z) = value {
					xform.translation.z = z;
				}
				reply!(command, "Ocean height: {}", xform.translation.z);
			}
			GetSetOceanValue::Seed { value } => {
				if let Some(seed) = value {
					let Some(mat) = mats.get_mut(mat.id()) else {
						reply_failed!(command, "couldn't get ocean material");
						return;
					};
					if seed.starts_with("rand") {
						mat.extension.seed = rand::random();
					} else {
						match seed.parse::<u32>() {
							Ok(seed) => mat.extension.seed = seed,
							Err(e) => reply_failed!(command, "failed to parse seed {seed:?}: {e}"),
						}
					}
					reply!(command, "{:?}", mat.extension.seed);
				} else {
					let Some(mat) = mats.get(mat.id()) else {
						reply_failed!(command, "couldn't get ocean material");
						return;
					};
					reply!(command, "{:?}", mat.extension.seed);
				}
			}
		}
	}
}

/// Adjust storm parameters
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "storm")]
#[clap(infer_subcommands = true)]
pub struct AdjustStorm {
	#[command(subcommand)]
	action: Option<GetSetStormValue>,
}

#[derive(Subcommand, Debug)]
pub enum GetSetStormValue {
	Intensity {
		#[arg(allow_hyphen_values = true)]
		value: Option<f32>,
	},
}

pub fn adjust_storm(mut command: ConsoleCommand<AdjustStorm>, mut storm: Option<ResMut<Storm>>) {
	if let Some(Ok(cmd)) = command.take() {
		let Some(storm) = storm.as_mut() else {
			reply_failed!(command, "Storm resource does not exist");
			return;
		};
		match cmd.action {
			None => reply!(command, "{:?}", &**storm),
			Some(GetSetStormValue::Intensity { value }) => {
				if let Some(intensity) = value {
					storm.intensity = intensity;
				}
				reply!(command, "{:?}", storm.intensity);
			}
		}
	}
}

/// Hide entities with name matching a glob pattern.
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "hide")]
pub struct Hide {
	glob: String,
}

/// Show entities with name matching a glob pattern.
#[derive(Parser, Debug, ConsoleCommand)]
#[command(name = "show")]
pub struct Show {
	glob: String,
	#[arg(
		short,
		long,
		default_value = "true",
		num_args = 0..=1,
		value_parser = [
			PossibleValue::new("true").aliases(["t", "yes", "y"]),
			PossibleValue::new("false").aliases(["f", "no", "n"]),
		]
	)]
	inherited: String,
}

pub trait SetVisibility {
	fn glob(&self) -> &str;
	fn set_visibility(&self, vis: &mut Visibility);
}

impl SetVisibility for Hide {
	fn glob(&self) -> &str {
		&self.glob
	}
	fn set_visibility(&self, vis: &mut Visibility) {
		*vis = Visibility::Hidden;
	}
}

impl SetVisibility for Show {
	fn glob(&self) -> &str {
		&self.glob
	}
	fn set_visibility(&self, vis: &mut Visibility) {
		if self.inherited.starts_with('t') || self.inherited.starts_with('y') {
			*vis = Visibility::Inherited;
		} else if self.inherited.starts_with('f') || self.inherited.starts_with('n') {
			*vis = Visibility::Visible;
		} else {
			unreachable!();
		}
	}
}

pub fn set_visibility<C: SetVisibility>(
	mut command: ConsoleCommand<C>,
	mut q: Query<(&Name, &mut Visibility)>,
) {
	if let Some(Ok(cmd)) = command.take() {
		let pat = match glob::Pattern::new(cmd.glob()) {
			Ok(p) => p,
			Err(e) => {
				reply_failed!(command, "{e}");
				return;
			}
		};
		let mut found = false;
		for (_, mut vis) in q.iter_mut().filter(|(name, _)| pat.matches(name.as_str())) {
			cmd.set_visibility(&mut vis);
			found = true;
		}
		if !found {
			reply_failed!(command, "couldn't find entity matching {:?}", cmd.glob());
		};
	}
}
