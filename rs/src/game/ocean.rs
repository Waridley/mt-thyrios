use crate::game::ocean::mesh::generate_ocean_mesh;
use crate::state::GlobalState;
use crate::util::GridMesh;
use bevy::color::palettes::css::MIDNIGHT_BLUE;
use bevy::math::Vec3A;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexBufferLayoutRef, VertexAttributeValues};
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError};
use rand::{random, Rng};
use std::f32::consts::{FRAC_PI_3, FRAC_PI_8, SQRT_2, TAU};
use GlobalState::InGame;

pub mod mesh;

const WAVE_COUNT: usize = 10;
const TIDE_COUNT: usize = 4;

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
	const RADIUS: f32 = 400.0;
	const RINGS: u32 = 400;
	const WEDGES: u32 = 1080;

	let mesh = generate_ocean_mesh(RADIUS, RINGS, WEDGES);

	let mut rng = rand::thread_rng();
	let mut rand_origin = move || {
		Vec2::new(
			(rng.gen::<f32>() * TAU * 2.0) - TAU,
			(rng.gen::<f32>() * TAU * 2.0) - TAU,
		)
	};
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
						amplitude: 1.0,
						steepness: 5.0,
						speed: 1.0,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.1,
						amplitude: 0.9,
						steepness: 4.0,
						speed: 1.0,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 0.15,
						amplitude: 0.8,
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
						steepness: 0.7,
						speed: 1.3,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 1.3,
						amplitude: 0.2,
						steepness: 0.5,
						speed: 1.7,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 1.7,
						amplitude: 0.1,
						steepness: 0.3,
						speed: 1.9,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 1.9,
						amplitude: 0.07,
						steepness: 0.3,
						speed: 2.3,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 2.3,
						amplitude: 0.05,
						steepness: 0.3,
						speed: 2.9,
					},
					Wave {
						origin: rand_origin(),
						direction: rand_direction(),
						frequency: 2.9,
						amplitude: 0.05,
						steepness: 0.3,
						speed: 3.7,
					},
				],
				tides: [
					Tide {
						// Every 7th tide is larger IRL
						frequency: 0.014285714285714287,
						amplitude: 1.2,
						steepness: 8.0,
						speed: 1.0,
					},
					Tide {
						frequency: 0.1,
						amplitude: 1.3,
						steepness: 2.0,
						speed: 1.0,
					},
					Tide {
						frequency: 0.15,
						amplitude: 1.1,
						steepness: 2.0,
						speed: 0.7,
					},
					Tide {
						frequency: 0.3,
						amplitude: 0.5,
						steepness: 0.5,
						speed: 0.5,
					},
				],
				size: RADIUS,
				storm_intensity: 0.4,
				horizon_color: Color::BLACK.to_linear().to_vec4(),
				
				/// Setting this to `false` enables an experimental stylized look without standard lighting.
				lighting: true,
				/// Toggles normal calculation from the vertex shader to the fragment shader.
				/// Fragment normals look better, but are slower to compute for the same mesh size.
				/// Vertex normals are linearly interpolated, thus looking worse at lower densities.
				/// A less-dense mesh with fragment normals on is usually faster for comparable quality
				/// than a denser mesh with fragment normals, but this should probably be abstracted
				/// into a quality option for player somehow.
				fragment_normals: true,
			},
			base: StandardMaterial {
				alpha_mode: AlphaMode::Blend,
				perceptual_roughness: 0.0,
				..default()
			},
		})),
		Aabb {
			center: Vec3A::ZERO,
			half_extents: Vec3A::splat(RADIUS * 0.5),
		},
	));
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct OceanSurface;

#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
#[bind_group_data(OceanShaderDefs)]
pub struct OceanMaterial {
	#[uniform(100)]
	pub waves: [Wave; WAVE_COUNT],
	#[uniform(100)]
	pub tides: [Tide; TIDE_COUNT],
	#[uniform(100)]
	pub size: f32,
	#[uniform(100)]
	pub storm_intensity: f32,
	#[uniform(100)]
	pub horizon_color: Vec4,
	
	pub lighting: bool,
	pub fragment_normals: bool,
}

impl MaterialExtension for OceanMaterial {
	fn vertex_shader() -> ShaderRef {
		"shaders/ocean.wgsl".into()
	}
	fn fragment_shader() -> ShaderRef {
		"shaders/ocean.wgsl".into()
	}
	fn specialize(
		_pipeline: &MaterialExtensionPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		key: MaterialExtensionKey<Self>
	) -> Result<(), SpecializedMeshPipelineError> {
		if key.bind_group_data.lighting {
			descriptor.vertex.shader_defs.push("LIGHTING".into());
			let fragment = descriptor.fragment.as_mut().unwrap();
			fragment.shader_defs.push("LIGHTING".into());
		}
		if key.bind_group_data.fragment_normals {
			descriptor.vertex.shader_defs.push("FRAGMENT_NORMALS".into());
			let fragment = descriptor.fragment.as_mut().unwrap();
			fragment.shader_defs.push("FRAGMENT_NORMALS".into());
		}
		Ok(())
	}
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct OceanShaderDefs {
	pub lighting: bool,
	pub fragment_normals: bool,
}

impl From<&OceanMaterial> for OceanShaderDefs {
	fn from(value: &OceanMaterial) -> Self {
		Self {
			lighting: value.lighting,
			fragment_normals: value.fragment_normals,
		}
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

#[derive(ShaderType, Debug, Clone, Copy, Reflect)]
pub struct Tide {
	pub frequency: f32,
	pub amplitude: f32,
	pub steepness: f32,
	pub speed: f32,
}
