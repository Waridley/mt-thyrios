use crate::game::mtn::MountainAssets;
use crate::new_game_setup_label;
use crate::setup_tracking::{AssetCollection, Progress, assets_progress};
use bevy::asset::{
	Asset, AssetServer, Handle, ReflectAsset, UntypedAssetId,
};
use bevy::image::Image;
use bevy::pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef};
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, ShaderDefVal, ShaderRef, SpecializedMeshPipelineError,
	VertexFormat,
};
use enum_map::{Enum, EnumMap};
use serde::{Deserialize, Serialize};
use strum::{EnumCount, FromRepr, VariantArray, VariantNames};

new_game_setup_label!(TerrainTexturesStacked, StackedTerrainTextures::progress);

#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	Eq,
	PartialEq,
	Hash,
	PartialOrd,
	Ord,
	EnumCount,
	FromRepr,
	VariantArray,
	VariantNames,
	Enum,
	Reflect,
	Serialize,
	Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
#[repr(u32)]
pub enum TerrainKind {
	#[default]
	Dirt,
	Sandstone,
	Limestone,
	Grass,
	Granite,
	Snow,
}

impl TryFrom<u32> for TerrainKind {
	type Error = u32;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		Self::from_repr(value).ok_or(value)
	}
}

impl TerrainKind {
	pub fn texture_path(self) -> &'static str {
		match self {
			TerrainKind::Dirt => "terrain/dirt.png",
			TerrainKind::Sandstone => "terrain/sandstone.png",
			TerrainKind::Limestone => "terrain/limestone.png",
			TerrainKind::Grass => "terrain/grass.png",
			TerrainKind::Granite => "terrain/granite.png",
			TerrainKind::Snow => "terrain/snow.png",
		}
	}
}

pub const ATTRIBUTE_TERRAIN_WEIGHTS_0_3: MeshVertexAttribute =
	MeshVertexAttribute::new("Terrain_Weights_0_3", 6400, VertexFormat::Unorm16x4);
pub const ATTRIBUTE_TERRAIN_WEIGHTS_4_7: MeshVertexAttribute =
	MeshVertexAttribute::new("Terrain_Weights_4_7", 6401, VertexFormat::Unorm16x4);

#[derive(Resource, Deref, Default)]
pub struct StackedTerrainTextures(pub EnumMap<TerrainKind, bool>);

impl StackedTerrainTextures {
	pub fn progress(done: Res<Self>) -> Progress {
		done.0[TerrainKind::Dirt].into()
	}
}

pub fn stack_terrain_textures(
	srv: Res<AssetServer>,
	map: Res<TexturesMap>,
	mut images: ResMut<Assets<Image>>,
	assets: Res<MountainAssets>,
	mut done: ResMut<StackedTerrainTextures>,
) {
	for kind in <TerrainKind as VariantArray>::VARIANTS {
		if !done.0[*kind] && srv.is_loaded_with_dependencies(map[*kind].id()) {
			info!("Updating texture for {kind:?}");
			let texture = images.get(map[*kind].id()).unwrap();
			let size = texture.size();
			let mut buf = vec![vec![Color::BLACK; size.x as usize]; size.y as usize];
			for y in 0..size.y {
				for x in 0..size.x {
					buf[y as usize][x as usize] = texture.get_color_at(x, y).unwrap();
				}
			}
			let textures = images.get_mut(assets.terrain_textures.id()).unwrap();
			for y in 0..size.y {
				for x in 0..size.x {
					textures
						.set_color_at(x, y + (size.y * *kind as u32), buf[y as usize][x as usize])
						.unwrap()
				}
			}
			done.0[*kind] = true;
		}
	}
}

#[derive(Resource, Deref, DerefMut)]
pub struct TexturesMap(EnumMap<TerrainKind, Handle<Image>>);

impl FromWorld for TexturesMap {
	fn from_world(world: &mut World) -> Self {
		let asset_server = world.get_resource::<AssetServer>().unwrap();
		Self(EnumMap::from_fn(|kind: TerrainKind| {
			asset_server.load(kind.texture_path())
		}))
	}
}

impl AssetCollection for TexturesMap {
	fn iter_ids(&self) -> impl Iterator<Item = UntypedAssetId> {
		// TODO: Actually return all handles when images are authored
		// self.0.values().map(|handle| handle.id().untyped())
		std::iter::once(self.0[TerrainKind::Dirt].id().untyped())
	}
}

new_game_setup_label!(TerrainTexturesLoaded, assets_progress::<TexturesMap>);

#[derive(AsBindGroup, Asset, Debug, Clone, Reflect)]
#[reflect(Asset)]
#[bind_group_data(TerrainShaderDefs)]
pub struct TerrainMaterial {
	#[texture(101, dimension = "2d_array")]
	#[sampler(102)]
	pub textures: Handle<Image>,
	// TODO: Graphics quality setting (triplanar mapping is 3x more expensive
	pub triplanar: bool,
}

impl TerrainMaterial {
	pub const SHADER_PATH: &'static str = "shaders/terrain.wgsl";
}

impl MaterialExtension for TerrainMaterial {
	fn vertex_shader() -> ShaderRef {
		Self::SHADER_PATH.into()
	}

	fn fragment_shader() -> ShaderRef {
		Self::SHADER_PATH.into()
	}

	fn specialize(
		_pipeline: &MaterialExtensionPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		layout: &MeshVertexBufferLayoutRef,
		key: MaterialExtensionKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.vertex.shader_defs.push(ShaderDefVal::UInt(
			"NUM_TERRAIN_KINDS".into(),
			TerrainKind::COUNT as u32,
		));
		if key.bind_group_data.triplanar {
			descriptor
				.fragment
				.as_mut()
				.unwrap()
				.shader_defs
				.push("TRIPLANAR".into());
		}
		let layout = layout.0.get_layout(&[
			Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
			Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
			ATTRIBUTE_TERRAIN_WEIGHTS_0_3.at_shader_location(8),
			ATTRIBUTE_TERRAIN_WEIGHTS_4_7.at_shader_location(9),
		])?;
		descriptor.vertex.buffers = vec![layout];
		Ok(())
	}
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct TerrainShaderDefs {
	triplanar: bool,
}

impl From<&TerrainMaterial> for TerrainShaderDefs {
	fn from(value: &TerrainMaterial) -> Self {
		Self {
			triplanar: value.triplanar,
		}
	}
}
