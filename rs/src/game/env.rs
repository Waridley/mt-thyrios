use crate::state::GlobalState::{self, InGame};
use bevy::prelude::*;
use bevy_atmosphere::model::AtmosphereModel;
use bevy_atmosphere::prelude::Nishita;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_environment)
			.insert_resource(AtmosphereModel::new(Nishita {
				ray_origin: Vec3::new(0.0, 0.0, 1001e3),
				sun_position: Vec3::new(1.0, -0.2, 1.0),
				planet_radius: 1000e3,
				atmosphere_radius: 1100e3,
				..default()
			}));
	}
}

pub fn setup_environment(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
) {
	cmds.spawn((
		Sunlight,
		DirectionalLight {
			illuminance: light_consts::lux::OVERCAST_DAY,
			..default()
		},
		Transform::from_rotation(Quat::from_rotation_arc(
			Vec3::NEG_Z,
			Vec3::new(-1.0, 0.2, -1.0).normalize(),
		)),
	));
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct Sunlight;
