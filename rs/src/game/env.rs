use bevy::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(AmbientLight {
			brightness: 10.0,
			..default()
		});
	}
}
