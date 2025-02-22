use crate::game::ocean::mesh::generate_ocean_mesh;
use crate::game::{GameSetupKey, GameSetupLabel};
use crate::new_game_setup_label;
use crate::setup_tracking::{IntoDependencyProvider, RegisterProvider, single_spawn_progress};
use crate::state::GlobalState;
use crate::util::MeshExt;
use GlobalState::{InGame, LoadingGame};
use bevy::math::{U16Vec2, Vec3A};
use bevy::pbr::{
	ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
	OpaqueRendererMethod,
};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderDefVal, ShaderRef, ShaderType,
	SpecializedMeshPipelineError,
};
use rand::Rng;
use std::f32::consts::TAU;

pub mod mesh;

const WAVE_COUNT: usize = 10;
const TIDE_COUNT: usize = 5;

pub struct OceanPlugin;

new_game_setup_label!(OceanSpawned, single_spawn_progress::<With<OceanSurface>>);

impl Plugin for OceanPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaterialPlugin::<
			ExtendedMaterial<StandardMaterial, OceanMaterial>,
		>::default())
			.insert_resource(StormIntensity(0.2))
			.register_provider(setup_ocean.provides([OceanSpawned.intern()]))
			.add_systems(Update, OceanMaterial::sync_storm_intensity);
	}
}

pub const RADIUS: f32 = 400.0;

pub fn setup_ocean(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
	storm_intensity: Res<StormIntensity>,
) {
	const RINGS: u32 = 300;
	const WEDGES: u32 = 1080;

	let mesh = generate_ocean_mesh(RADIUS, RINGS, WEDGES);
	let ocean_verts = mesh.positions().unwrap().len();
	let ocean_tris = mesh.indices().unwrap().len() / 3;
	debug!(ocean_verts, ocean_tris);

	// TODO: Switch rand 0.9?
	//    (`Rng::gen` was renamed to `Rng::random` for compatibility with edition 2024)
	let mut rng = rand::thread_rng();
	let mut rand_origin = move || {
		Vec2::new(
			(rng.r#gen::<f32>() * TAU * 2.0) - TAU,
			(rng.r#gen::<f32>() * TAU * 2.0) - TAU,
		)
	};
	let mut rng = rand::thread_rng();
	let mut rand_direction = move || Vec2::from_angle(rng.r#gen::<f32>() * TAU);
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
						amplitude: 2.0,
						steepness: 3.0,
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
				base_tide: Tide {
					frequency: 0.01,
					amplitude: 3.0,
					steepness: 1.0,
					speed: 0.1,
				},
				vertex_wave_octaves: 8,
				vertex_tide_octaves: 4,
				fragment_wave_octaves: 10,
				fragment_tide_octaves: 5,
				size: RADIUS,
				storm_intensity: **storm_intensity,
				horizon_color: Color::BLACK.to_linear().to_vec4(),

				lighting: false,
				fragment_normals: true,
			},
			base: StandardMaterial {
				// NOTE: This needs set to Opaque if ScreenSpaceReflections get enabled
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
	pub base_tide: Tide,
	#[uniform(100)]
	pub vertex_wave_octaves: u32,
	#[uniform(100)]
	pub vertex_tide_octaves: u32,
	#[uniform(100)]
	pub fragment_wave_octaves: u32,
	#[uniform(100)]
	pub fragment_tide_octaves: u32,
	#[uniform(100)]
	pub size: f32,
	#[uniform(100)]
	pub storm_intensity: f32,
	#[uniform(100)]
	pub horizon_color: Vec4,

	/// Setting this to `false` enables an experimental stylized look without standard lighting.
	pub lighting: bool,
	/// Toggles normal calculation from the vertex shader to the fragment shader.
	/// Fragment normals look better, but are slower to compute for the same mesh size.
	/// Vertex normals are linearly interpolated, thus looking worse at lower densities.
	/// A less-dense mesh with fragment normals on is usually faster for comparable quality
	/// than a denser mesh with fragment normals, but this should probably be abstracted
	/// into a quality option for player somehow.
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
		key: MaterialExtensionKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		let wave_count = ShaderDefVal::UInt("WAVE_COUNT".into(), WAVE_COUNT as u32);
		let tide_count = ShaderDefVal::UInt("TIDE_COUNT".into(), TIDE_COUNT as u32);
		descriptor.vertex.shader_defs.push(wave_count.clone());
		descriptor.vertex.shader_defs.push(tide_count.clone());
		let fragment = descriptor.fragment.as_mut().unwrap();
		fragment.shader_defs.push(wave_count);
		fragment.shader_defs.push(tide_count);
		if key.bind_group_data.lighting {
			descriptor.vertex.shader_defs.push("LIGHTING".into());
			let fragment = descriptor.fragment.as_mut().unwrap();
			fragment.shader_defs.push("LIGHTING".into());
		}
		if key.bind_group_data.fragment_normals {
			descriptor
				.vertex
				.shader_defs
				.push("FRAGMENT_NORMALS".into());
			let fragment = descriptor.fragment.as_mut().unwrap();
			fragment.shader_defs.push("FRAGMENT_NORMALS".into());
		}
		Ok(())
	}
}

impl OceanMaterial {
	pub fn sync_storm_intensity(
		mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
		intensity: Res<StormIntensity>,
	) {
		if intensity.is_changed() {
			for (_, mat) in mats.iter_mut() {
				mat.extension.storm_intensity = **intensity;
			}
		}
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

#[derive(ShaderType, Debug, Clone, Copy, Reflect)]
pub struct Fbm {
	pub vertex_wave_octaves: u32,
	pub vertex_tide_octaves: u32,
	pub fragment_wave_octaves: u32,
	pub fragment_tide_octaves: u32,
}

#[derive(Resource, Debug, Deref, DerefMut)]
pub struct StormIntensity(pub f32);
