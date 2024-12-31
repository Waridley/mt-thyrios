use bevy::color::palettes::css::MIDNIGHT_BLUE;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use rand::random;
use GlobalState::InGame;
use crate::state::GlobalState;

pub struct OceanPlugin;

impl Plugin for OceanPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_ocean);
	}
}

pub fn setup_ocean(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>
) {
	let mut mesh = Plane3d::new(Vec3::Z, Vec2::splat(60.0))
		.mesh()
		.subdivisions(1_000)
		.build();
	
	match mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION).unwrap() {
		VertexAttributeValues::Float32x3(val) => {
			for pos in val {
				pos[2] += random::<f32>() * 0.1;
			}
		}
		_ => unreachable!(),
	}
	
	mesh.compute_normals();
	
	cmds.spawn((
		OceanSurface,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d(mats.add(StandardMaterial {
			..StandardMaterial::from(Color::from(MIDNIGHT_BLUE.with_alpha(0.2)))
		})),
		StateScoped(InGame),
	));
}

#[derive(Component, Debug)]
pub struct OceanSurface;
