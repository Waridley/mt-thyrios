use crate::state::GlobalState::LoadingGame;
use crate::{
	game::tools::{ReflectTool, Tool},
	state::GlobalState,
};
use bevy::asset::{weak_handle, RenderAssetUsages};
use bevy::ecs::query::QueryData;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
	AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
	TextureDimension, TextureFormat,
};

pub struct PlacementPlugin;

pub const DEFAULT_INTERSECTION_DEPTH_MAP_HANDLE: Handle<Image> =
	weak_handle!("c2fb1c89-954d-4285-92c6-be35d09cfcbe");

impl Plugin for PlacementPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaterialPlugin::<MtnCursorMaterial>::default())
			.register_type::<PlaceBuilding>()
			.add_systems(OnEnter(LoadingGame), MtnCursor::spawn);
	}

	fn finish(&self, app: &mut App) {
		let mut images = app.world_mut().resource_mut::<Assets<Image>>();
		let img = new_intersection_depth_map(64, |t| {
			Color::WHITE.with_alpha(1.0 - (t * 5.0).clamp(0.0, 1.0).powf(0.25))
		});
		images.insert(DEFAULT_INTERSECTION_DEPTH_MAP_HANDLE.id(), img);
	}
}

#[derive(Component)]
#[require(Transform, Visibility = Visibility::Hidden, Mesh3d, MeshMaterial3d<MtnCursorMaterial>, StateScoped<GlobalState> = StateScoped(GlobalState::InGame))]
pub struct MtnCursor;

impl MtnCursor {
	pub fn spawn(mut cmds: Commands) {
		cmds.spawn(Self);
	}
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct MtnCursorQuery {
	pub entity: Entity,
	pub cursor: &'static mut MtnCursor,
	pub xform: &'static mut Transform,
	pub global_xform: &'static mut GlobalTransform,
	pub visibility: &'static mut Visibility,
	pub mesh: &'static mut Mesh3d,
	pub material: &'static mut MeshMaterial3d<MtnCursorMaterial>,
}

#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct MtnCursorMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
	#[texture(1)]
	#[sampler(2)]
	pub base_color_texture: Option<Handle<Image>>,
	#[texture(3, dimension = "1d")]
	#[sampler(4)]
	pub intersection_color_map: Handle<Image>,
	#[uniform(0)]
	pub intersection_depth_mul: f32,
}

impl Default for MtnCursorMaterial {
	fn default() -> Self {
		MtnCursorMaterial {
			base_color: LinearRgba::WHITE.with_alpha(0.01).to_vec4(),
			base_color_texture: None,
			intersection_color_map: DEFAULT_INTERSECTION_DEPTH_MAP_HANDLE.clone(),
			intersection_depth_mul: 1.0,
		}
	}
}

impl Material for MtnCursorMaterial {
	fn fragment_shader() -> ShaderRef {
		"shaders/mtn_cursor.wgsl".into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Premultiplied
	}

	fn specialize(
		_pipeline: &MaterialPipeline<Self>,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}

pub fn new_intersection_depth_map(size: u32, map: impl Fn(f32) -> Color) -> Image {
	let mut img = Image::new_fill(
		Extent3d {
			width: size,
			height: 1,
			..default()
		},
		TextureDimension::D1,
		&[0; 16],
		TextureFormat::Rgba32Float,
		RenderAssetUsages::all(),
	);
	for i in 0..size {
		img.set_color_at_1d(i, map(i as f32 / (size - 1) as f32))
			.unwrap()
	}
	img
}

#[derive(Debug, Reflect)]
#[reflect(Tool)]
pub struct PlaceBuilding {}

impl Tool for PlaceBuilding {}
