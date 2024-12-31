use bevy::prelude::*;
use crate::state::GlobalState::InGame;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_environment);
	}
}

pub fn setup_environment(mut cmds: Commands) {
	cmds.spawn((
		Sunlight,
		DirectionalLight::default(),
		Transform::from_rotation(Quat::from_rotation_arc(
			Vec3::NEG_Z,
			Vec3::new(-1.0, 0.2, -1.0),
		)),
		StateScoped(InGame),
	));
}

#[derive(Component, Debug)]
pub struct Sunlight;
