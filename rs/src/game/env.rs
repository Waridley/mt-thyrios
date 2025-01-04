use bevy::prelude::*;
use crate::state::GlobalState::InGame;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_environment);
	}
}

pub fn setup_environment(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>
) {
	cmds.insert_resource(AmbientLight {
		brightness: 2000.0,
		..default()
	});
	cmds.spawn((
		Sunlight,
		DirectionalLight::default(),
		Transform::from_rotation(Quat::from_rotation_arc(
			Vec3::NEG_Z,
			Vec3::new(-1.0, 0.2, -1.0).normalize(),
		)),
		StateScoped(InGame),
	)).with_child((
		Sun,
		Mesh3d(meshes.add(Sphere::new(2.0).mesh().build())),
		MeshMaterial3d(mats.add(StandardMaterial {
			base_color: Color::WHITE,
			emissive: LinearRgba::WHITE * 100.0,
			..default()
		})),
		Transform::from_translation(Vec3::Z * 100.0),
		StateScoped(InGame),
	));
}

#[derive(Component, Debug)]
pub struct Sun;

#[derive(Component, Debug)]
pub struct Sunlight;
