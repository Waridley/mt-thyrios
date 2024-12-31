use std::f32::consts::TAU;
use bevy::color::palettes::css::DARK_GREEN;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::VertexFormat;
use rand::random;
use GlobalState::InGame;
use crate::state::GlobalState;

pub struct MountainPlugin;

impl Plugin for MountainPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), spawn_mountain);
	}
}

pub fn spawn_mountain(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>
) {
	let mut mesh = ConicalFrustum {
		radius_top: 2.0,
		radius_bottom: 20.0,
		height: 20.0,
	}.mesh()
		.resolution(4_000)
		.segments(200)
		.build();
	for (_, vals) in mesh.attributes_mut() {
		match vals {
			VertexAttributeValues::Float32x3(vec3s) => for vec3 in vec3s {
				let y_up = Vec3::from_array(*vec3);
				let mut z_up = Vec3::new(y_up.x, -y_up.z, y_up.y);
				*vec3 = z_up.to_array()
			}
			_ => {}
		}
	}
	
	// Fake grass for seeing rotation during prototyping
	match mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL).unwrap() {
		VertexAttributeValues::Float32x3(vec3s) => {
			for mut norm in vec3s {
				let mut n = Vec3::from_array(*norm);
				n.x += random::<f32>() * 0.4 - 0.2;
				n.y += random::<f32>() * 0.4 - 0.2;
				n.z = random::<f32>() * 0.5;
				*norm = n.normalize().to_array();
			}
		}
		_ => unreachable!(),
	}
	
	cmds.spawn((
		Mountain,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d::<StandardMaterial>(mats.add(StandardMaterial {
			base_color: Color::linear_rgb(0.0, 0.004, 0.0),
			reflectance: 0.25,
			perceptual_roughness: 0.9,
			..default()
		})),
		StateScoped(InGame),
	));
}

#[derive(Component, Debug)]
pub struct Mountain;
