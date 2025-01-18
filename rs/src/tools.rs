use crate::game::ocean::{OceanMaterial, OceanSurface};
use bevy::color::palettes::basic::YELLOW;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, (toggle_fps, update_fps, adjust_storm));
	}
}

pub fn toggle_fps(
	mut cmds: Commands,
	q: Option<Single<Entity, With<FpsText>>>,
	mut keys: Res<ButtonInput<KeyCode>>,
) {
	if keys.just_pressed(KeyCode::F10) {
		if let Some(entity) = q {
			cmds.entity(*entity).despawn_recursive();
		} else {
			cmds.spawn((FpsText, Text("FPS: ...".into()), TextColor(YELLOW.into())));
		}
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
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
	mut ocean: Single<&mut Transform, With<OceanSurface>>,
	keys: Res<ButtonInput<KeyCode>>,
	t: Res<Time>,
) {
	let mut incr = 0.0;
	if keys.pressed(KeyCode::Equal) {
		incr += t.delta_secs();
	}
	if keys.pressed(KeyCode::Minus) {
		incr -= t.delta_secs();
	}

	if incr != 0.0 {
		for (_, mat) in mats.iter_mut() {
			let intensity = (mat.extension.storm_intensity + incr * 0.5).clamp(0.0, 2.0);
			if mat.extension.storm_intensity != intensity {
				mat.extension.storm_intensity = intensity;
				info!(intensity);
			}
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
