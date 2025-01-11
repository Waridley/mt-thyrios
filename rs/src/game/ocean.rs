use crate::state::GlobalState;
use crate::util::GridMesh;
use bevy::color::palettes::css::MIDNIGHT_BLUE;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use rand::{random, Rng};
use std::f32::consts::{FRAC_PI_3, FRAC_PI_8, SQRT_2, TAU};
use bevy::math::Vec3A;
use bevy::render::primitives::Aabb;
use GlobalState::InGame;

pub struct OceanPlugin;

impl Plugin for OceanPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaterialPlugin::<
			ExtendedMaterial<StandardMaterial, OceanMaterial>,
		>::default())
			.add_systems(OnEnter(InGame), setup_ocean);
	}
}

pub fn setup_ocean(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
) {
	const SIZE: f32 = 600.0;
	const SUBDIVS: u32 = 1500;

	let mut mesh = Circle::new(SIZE).grid_mesh().subdivisions(SUBDIVS).build();
	let verts = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().len();
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Vec4::ONE; verts]);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![Vec3::Z; verts]);
	
	let mut rng = rand::thread_rng();
	let mut rand_origin = move || Vec2::new(
		(rng.gen::<f32>() * TAU * 2.0) - TAU,
		(rng.gen::<f32>() * TAU * 2.0) * TAU
	);
	let mut rng = rand::thread_rng();
	let mut rand_direction = move || Vec2::from_angle(rng.gen::<f32>() * TAU);
	cmds.spawn((
		OceanSurface,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d(mats.add(ExtendedMaterial {
			extension: OceanMaterial {
				waves: [
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.03,
						amplitude: 1.2,
						steepness: 5.0,
						speed: 1.0,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.1,
						amplitude: 1.3,
						steepness: 4.0,
						speed: 1.0,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.15,
						amplitude: 1.1,
						steepness: 2.0,
						speed: 0.7,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.3,
						amplitude: 0.7,
						steepness: 1.0,
						speed: 1.1,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.7,
						amplitude: 0.3,
						steepness: 0.5,
						speed: 1.3,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 1.3,
						amplitude: 0.2,
						steepness: 0.25,
						speed: 1.7,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 1.9,
						amplitude: 0.1,
						steepness: 0.1,
						speed: 1.9,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 2.9,
						amplitude: 0.07,
						steepness: 0.1,
						speed: 2.3,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 3.7,
						amplitude: 0.05,
						steepness: 0.1,
						speed: 2.9,
					},
				// 	Wave {
				// 		origin: rand_origin(),
				// 		direction: rand_direction(),
				// 		frequency: 4.1,
				// 		amplitude: 0.05,
				// 		steepness: 0.1,
				// 		speed: 3.7,
				// 	},
				],
				size: SIZE,
				storm_intensity: 0.4,
			},
			base: StandardMaterial {
				alpha_mode: AlphaMode::Blend,
				perceptual_roughness: 0.0,
				double_sided: true,
				cull_mode: None,
				..default()
			},
		})),
		Aabb {
			center: Vec3A::ZERO,
			half_extents: Vec3A::splat(SIZE * 0.5),
		},
	));
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct OceanSurface;

const WAVE_COUNT: usize = 9;

#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
pub struct OceanMaterial {
	#[uniform(100)]
	pub waves: [Wave; WAVE_COUNT],
	#[uniform(100)]
	pub size: f32,
	#[uniform(100)]
	pub storm_intensity: f32,
}

impl MaterialExtension for OceanMaterial {
	fn vertex_shader() -> ShaderRef {
		"shaders/ocean.wgsl".into()
	}
	fn fragment_shader() -> ShaderRef {
		"shaders/ocean.wgsl".into()
	}
}

#[derive(ShaderType, Debug, Clone, Copy, Reflect)]
pub struct Wave {
	pub origin: Vec2,
	pub direction: Vec2,
	pub frequency: f32,
	pub amplitude: f32,
	pub steepness: f32,
	pub speed: f32,
}
