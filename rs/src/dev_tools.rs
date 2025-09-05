use crate::game::ocean::{OceanSurface, Storm};
use crate::state::GlobalState::InGame;
use bevy::color::palettes::basic::YELLOW;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::input::common_conditions::input_toggle_active;
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use console::DevConsolePlugin;
use bird_barrier::{toggle_setup_graph_window, SetupGraphVisualizationPlugin};
use crate::game::GameSetupKey;

pub mod console;
pub mod log_view;
pub mod paint;
// pub mod setup_graph_vis;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			paint::TerrainPaintPlugin,
			WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::KeyI)),
			DevConsolePlugin,
			SetupGraphVisualizationPlugin::<GameSetupKey>::default(),
		))
		.add_systems(
			Update,
			(toggles, update_fps, adjust_storm.run_if(in_state(InGame))),
		);
	}
}

pub fn toggles(
	mut cmds: Commands,
	keys: Res<ButtonInput<KeyCode>>,
	q: Option<Single<Entity, With<FpsText>>>,
	mut wireframe: ResMut<WireframeConfig>,
	egui_wants_input: Res<EguiWantsInput>,
) {
	if egui_wants_input.wants_any_keyboard_input() {
		return;
	}
	
	if keys.just_pressed(KeyCode::F10) {
		info!("Toggling FPS text");
		if let Some(entity) = q {
			cmds.entity(*entity).despawn();
		} else {
			cmds.spawn((FpsText, Text("FPS: ...".into()), TextColor(YELLOW.into())));
		}
	}
	if keys.just_pressed(KeyCode::F9) {
		info!("Toggling wireframes");
		wireframe.global = !wireframe.global;
	}
	if keys.just_pressed(KeyCode::KeyG) {
		info!("Toggling setup graph window");
		cmds.run_system_cached(toggle_setup_graph_window::<GameSetupKey>);
	}
}

pub fn update_fps(mut q: Option<Single<&mut Text, With<FpsText>>>, diags: Res<DiagnosticsStore>) {
	if let Some(text) = q.as_mut() {
		let Some(fps) = diags.get(&FrameTimeDiagnosticsPlugin::FPS) else {
			return;
		};
		let Some(fps) = fps.smoothed() else { return };
		text.0 = format!("FPS: {fps:>8.3}");
	}
}

#[derive(Component, Debug)]
pub struct FpsText;

pub fn adjust_storm(
	mut storm: ResMut<Storm>,
	mut ocean: Single<&mut Transform, With<OceanSurface>>,
	keys: Res<ButtonInput<KeyCode>>,
	t: Res<Time>,
	egui_wants_input: Res<EguiWantsInput>,
) {
	if egui_wants_input.wants_any_keyboard_input() {
		return;
	}
	
	let mut incr = 0.0;
	if keys.pressed(KeyCode::Equal) {
		incr += t.delta_secs();
	}
	if keys.pressed(KeyCode::Minus) {
		incr -= t.delta_secs();
	}

	if incr != 0.0 {
		let new_intensity = (storm.intensity + incr * 0.5).clamp(0.0, 2.0);
		if storm.intensity != new_intensity {
			storm.intensity = new_intensity;
			info!(new_intensity);
		}
	}

	let mut incr = 0.0;
	if keys.pressed(KeyCode::PageUp) {
		incr += t.delta_secs();
	}
	if keys.pressed(KeyCode::PageDown) {
		incr -= t.delta_secs();
	}

	if incr != 0.0 {
		ocean.translation.z += incr * 5.0;
		let ocean_z = ocean.translation.z;
		info!(ocean_z);
	}

	if keys.just_pressed(KeyCode::Home) {
		ocean.translation.z = 0.0;
	}
}
