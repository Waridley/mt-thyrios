use crate::state::GlobalState;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use rand::random;
use GlobalState::InGame;

pub struct MountainPlugin;

impl Plugin for MountainPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), spawn_mountain);
	}
}

pub fn spawn_mountain(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
) {
	const HEIGHT: f32 = 150.0;
	let mut mesh = ConicalFrustum {
		radius_top: 10.0,
		radius_bottom: 300.0,
		height: HEIGHT,
	}
	.mesh()
	.resolution(4_000)
	.segments(200)
	.build();
	for (_, vals) in mesh.attributes_mut() {
		match vals {
			VertexAttributeValues::Float32x3(vec3s) => {
				for vec3 in vec3s {
					let y_up = Vec3::from_array(*vec3);
					let mut z_up = Vec3::new(y_up.x, -y_up.z, y_up.y);
					*vec3 = z_up.to_array()
				}
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

	let colors = match mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap() {
		VertexAttributeValues::Float32x3(positions) => positions
			.iter()
			.map(|v| {
				let l = (v[2] + (HEIGHT * 0.5)) / HEIGHT;
				let curve = EasingCurve::new(0.0, 1.0, EaseFunction::QuinticIn);
				let l = curve.sample(l).unwrap();
				let l = curve.sample(l).unwrap();
				Vec4::new(
					f32::min(l + 00.01, 1.0),
					f32::min(l + 0.04, 1.0),
					f32::min(l + 0.004, 1.0),
					1.0
				)
			})
			.collect::<Vec<_>>(),
		_ => unreachable!(),
	};
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

	cmds.spawn((
		Mountain,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d::<StandardMaterial>(mats.add(StandardMaterial {
			reflectance: 0.25,
			perceptual_roughness: 0.9,
			..default()
		})),
		Transform {
			..default()
		},
	));
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct Mountain;
