use crate::game::cam::CamAnchor;
use crate::game::GameSetupLabel;
use crate::new_game_setup_label;
use crate::setup_tracking::{IntoDependencyProvider, RegisterProvider, single_spawn_progress};
use crate::state::GlobalState;
use crate::util::{GridMesh, MeshExt};
use GlobalState::InGame;
use bevy::asset::ReflectAsset;
use bevy::math::Vec3A;
use bevy::pbr::{
	ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType,
	SpecializedMeshPipelineError,
};

pub struct OceanPlugin;

new_game_setup_label!(OceanSpawned, single_spawn_progress::<With<OceanSurface>>);

impl Plugin for OceanPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaterialPlugin::<
			ExtendedMaterial<StandardMaterial, OceanMaterial>,
		>::default())
			.register_asset_reflect::<ExtendedMaterial<StandardMaterial, OceanMaterial>>()
			.insert_resource(Storm { intensity: 0.2 })
			.register_provider(setup_ocean.provides([OceanSpawned.intern()]))
			.add_systems(
				Update,
				(
					OceanMaterial::sync_storm_intensity,
					ocean_rotation_follow_cam_anchor.after(crate::game::input::cam_input),
				)
					.run_if(in_state(InGame)),
			);
	}
}

pub const RADIUS: f32 = 400.0;

pub fn setup_ocean(
	mut cmds: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, OceanMaterial>>>,
	storm: Res<Storm>,
) {
	// TODO: Graphics quality setting
	const SUBDIVS: u32 = 1024;

	// TODO PERF: make a baseball field-shaped mesh with larger triangles further away
	let mut mesh = Circle::new(RADIUS)
		.grid_mesh()
		.subdivisions(SUBDIVS)
		// Order back-to-front since the transparent mesh pipeline does not do z buffer culling
		.map_vertices(|v: Vec2| Vec3::new(v.x, -v.y, 0.0))
		.build()
		.with_inverted_winding()
		.unwrap();

	let verts = mesh.count_vertices();
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0; 4]; verts]);
	let ocean_verts = mesh.positions().unwrap().len();
	let ocean_tris = mesh.indices().unwrap().len() / 3;
	debug!(ocean_verts, ocean_tris);

	let seed = rand::random::<u32>();
	debug!(seed);
	cmds.spawn((
		Name::new("OceanSurface"),
		OceanSurface,
		Mesh3d(meshes.add(mesh)),
		MeshMaterial3d(mats.add(ExtendedMaterial {
			extension: OceanMaterial {
				seed,
				vertex_wave_octaves: 8,
				vertex_tide_octaves: 4,
				fragment_wave_octaves: 10,
				fragment_tide_octaves: 5,
				size: RADIUS,
				storm_intensity: storm.intensity,
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
#[require(StateScoped<GlobalState> = StateScoped(InGame))]
pub struct OceanSurface;

#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
#[reflect(Asset)]
#[bind_group_data(OceanShaderDefs)]
pub struct OceanMaterial {
	#[uniform(100)]
	pub seed: u32,
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
		storm: Res<Storm>,
	) {
		if storm.is_changed() {
			for (_, mat) in mats.iter_mut() {
				mat.extension.storm_intensity = storm.intensity;
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

#[derive(Resource, Debug)]
pub struct Storm {
	pub intensity: f32,
}

/// A bit of a hack to keep triangles ordered back-to-front from the camera's view.
pub fn ocean_rotation_follow_cam_anchor(
	anchor: Single<Ref<GlobalTransform>, With<CamAnchor>>,
	mut ocean: Single<&mut Transform, With<OceanSurface>>,
) {
	if anchor.is_changed() {
		ocean.rotation = anchor.rotation();
	}
}
