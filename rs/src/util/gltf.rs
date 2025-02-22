use crate::util::ZUp;
use bevy::asset::io::Writer;
use bevy::asset::saver::{AssetSaver, SavedAsset};
use bevy::asset::transformer::{AssetTransformer, TransformedAsset};
use bevy::asset::{
	AssetLoader, AsyncWriteExt, ErasedLoadedAsset, RenderAssetUsages, UntypedAssetId,
};
use bevy::gltf::{GltfLoader, GltfLoaderSettings, GltfMesh, GltfNode};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexAttribute};
use bevy::render::render_resource::VertexFormat;
use bevy::utils::{ConditionalSendFuture, HashMap, HashSet};
use gltf::Glb;
use gltf::binary::Header;
use gltf_json::accessor::GenericComponentType;
use gltf_json::buffer::{Stride, View};
use gltf_json::extras::RawValue;
use gltf_json::mesh::{Primitive, Semantic};
use gltf_json::scene::UnitQuaternion;
use gltf_json::validation::Checked::Valid;
use gltf_json::validation::{Checked, USize64};
use gltf_json::{Accessor, Extras, Index};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::num::TryFromIntError;

pub struct GlbSaver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlbSaverSettings {
	pub load_meshes: RenderAssetUsages,
	pub load_materials: RenderAssetUsages,
	pub load_cameras: bool,
	pub load_lights: bool,
	pub include_source: bool,
}

impl Default for GlbSaverSettings {
	fn default() -> Self {
		let GltfLoaderSettings {
			load_meshes,
			load_materials,
			load_cameras,
			load_lights,
			include_source,
		} = GltfLoaderSettings::default();
		Self {
			load_meshes,
			load_materials,
			load_cameras,
			load_lights,
			include_source,
		}
	}
}

impl AssetSaver for GlbSaver {
	type Asset = Gltf;
	type Settings = GlbSaverSettings;
	type OutputLoader = GltfLoader;
	type Error = GlbSaverError;

	async fn save(
		&self,
		writer: &mut Writer,
		asset: SavedAsset<'_, Self::Asset>,
		settings: &Self::Settings,
	) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
		let &GlbSaverSettings {
			load_meshes,
			load_materials,
			load_cameras,
			load_lights,
			include_source,
		} = settings;

		let gltf = &*asset;
		let deps = asset
			.iter_labels()
			.map(|label| (asset.get_untyped_handle(label).unwrap().id(), label))
			.collect::<HashMap<UntypedAssetId, &str>>();

		// First push all data to `bytes`, keeping track of views and offsets, etc.
		let mut bytes = Vec::new();

		let pending_meshes = gltf
			.meshes
			.iter()
			.map(|mesh| append_gltf_mesh(&mut bytes, &asset, mesh.id(), &deps))
			.collect::<Vec<_>>();

		// Ensure padding to a multiple of 4 bytes
		while bytes.len() % 4 != 0 {
			bytes.push(0);
		}

		// Then build json document now that we know the length of `bytes`
		let mut root = gltf_json::Root::default();
		// let mut bevy_mesh_to_parent_gltf_mesh_map = HashMap::new();
		let buffer = root.push(gltf_json::Buffer {
			byte_length: bytes.len().into(),
			name: None,
			uri: None,
			extensions: None,
			extras: None,
		});
		let gltf_meshes = pending_meshes
			.into_iter()
			.map(|pending_gltf_mesh| {
				let (bevy_mesh_ids, primitives) = pending_gltf_mesh
					.primitives
					.into_iter()
					.map(|(pending_attrs, pending_primitive)| {
						let bevy_mesh_id = pending_primitive.id;
						let PendingAttributes {
							attributes,
							indices_view,
							indices_accessor,
						} = pending_attrs;
						let attributes = attributes
							.into_iter()
							.map(|(name, (view, accessor))| {
								let view = root.push(View {
									buffer,
									byte_length: view.byte_length,
									byte_offset: view.byte_offset,
									byte_stride: view.byte_stride,
									name: view.name,
									target: view.target,
									extensions: view.extensions,
									extras: view.extras,
								});
								let accessor = root.push(Accessor {
									buffer_view: Some(view),
									byte_offset: Some(USize64(0)),
									count: accessor.count,
									component_type: accessor.component_type,
									extensions: accessor.extensions,
									extras: accessor.extras,
									type_: accessor.type_,
									min: accessor.min,
									max: accessor.max,
									name: accessor.name,
									normalized: accessor.normalized,
									sparse: accessor.sparse,
								});
								(name, accessor)
							})
							.collect::<BTreeMap<_, _>>();

						let indices = indices_view.zip(indices_accessor).map(|(view, accessor)| {
							let view = root.push(View {
								buffer,
								byte_length: view.byte_length,
								byte_offset: view.byte_offset,
								byte_stride: view.byte_stride,
								name: view.name,
								target: view.target,
								extensions: view.extensions,
								extras: view.extras,
							});
							root.push(Accessor {
								buffer_view: Some(view),
								byte_offset: Some(USize64(0)),
								count: accessor.count,
								component_type: accessor.component_type,
								extensions: accessor.extensions,
								extras: accessor.extras,
								type_: accessor.type_,
								min: accessor.min,
								max: accessor.max,
								name: accessor.name,
								normalized: accessor.normalized,
								sparse: accessor.sparse,
							})
						});

						let primitive = Primitive {
							attributes,
							extensions: pending_primitive.extensions,
							extras: pending_primitive.extras,
							indices,
							material: pending_primitive.material,
							mode: pending_primitive.mode,
							targets: pending_primitive.targets,
						};
						(bevy_mesh_id, primitive)
					})
					.unzip::<_, _, Vec<_>, Vec<_>>();

				let mesh = root.push(gltf_json::Mesh {
					extensions: pending_gltf_mesh.mesh.extensions,
					extras: pending_gltf_mesh.mesh.extras,
					name: pending_gltf_mesh.mesh.name,
					primitives,
					weights: pending_gltf_mesh.mesh.weights,
				});
				// bevy_mesh_to_parent_gltf_mesh_map
				// 	.extend(bevy_mesh_ids.into_iter().map(|id| (id, mesh)));
				(pending_gltf_mesh.id, mesh)
			})
			.collect::<HashMap<_, _>>();

		let _scenes = gltf
			.scenes
			.iter()
			.map(|scene| push_scene(&mut root, &asset, scene.id(), &deps, &gltf_meshes))
			.collect::<Vec<_>>();

		// TODO: Materials, animations, skins, morph targets

		let json_bytes = gltf_json::serialize::to_vec(&root)?;

		let glb = Glb {
			header: Header {
				magic: *b"glTF",
				version: 2,
				length: (json_bytes.len() + bytes.len()).try_into()?,
			},
			json: Cow::Owned(json_bytes),
			bin: Some(Cow::Owned(bytes)),
		};
		let bytes = glb.to_vec()?;
		writer.write_all(&bytes).await?;

		Ok(GltfLoaderSettings {
			load_meshes,
			load_materials,
			load_cameras,
			load_lights,
			include_source,
		})
	}
}

#[must_use]
fn append_gltf_mesh(
	bytes: &mut Vec<u8>,
	asset: &SavedAsset<Gltf>,
	id: AssetId<GltfMesh>,
	deps: &HashMap<UntypedAssetId, &str>,
) -> PendingGltfMesh {
	let mesh = &*asset
		.get_labeled::<GltfMesh, _>(deps[&id.untyped()])
		.unwrap();
	let primitives = mesh
		.primitives
		.iter()
		.map(|primitive| {
			let mesh = &*asset
				.get_labeled::<Mesh, _>(deps[&primitive.mesh.id().untyped()])
				.unwrap();
			let attrs = append_mesh(bytes, mesh);

			(
				attrs,
				PendingPrimitive {
					id: primitive.mesh.id(),
					extensions: None,
					extras: gltf_json_extras(&primitive.extras),
					material: None,
					mode: Default::default(),
					targets: None,
				},
			)
		})
		.collect::<Vec<_>>();
	PendingGltfMesh {
		primitives,
		mesh: PendingMesh {
			extensions: None,
			extras: gltf_json_extras(&mesh.extras),
			name: Some(mesh.name.clone()),
			weights: None,
		},
		id,
	}
}

struct PendingGltfMesh {
	id: AssetId<GltfMesh>,
	primitives: Vec<(PendingAttributes, PendingPrimitive)>,
	mesh: PendingMesh,
}

struct PendingPrimitive {
	id: AssetId<Mesh>,
	extensions: Option<gltf_json::extensions::mesh::Primitive>,
	extras: Extras,
	material: Option<Index<gltf_json::material::Material>>,
	mode: Checked<gltf_json::mesh::Mode>,
	targets: Option<Vec<gltf_json::mesh::MorphTarget>>,
}

struct PendingMesh {
	extensions: Option<gltf_json::extensions::mesh::Mesh>,
	extras: Extras,
	name: Option<String>,
	weights: Option<Vec<f32>>,
}

#[must_use]
fn append_mesh(bytes: &mut Vec<u8>, mesh: &Mesh) -> PendingAttributes {
	let mut attributes = BTreeMap::new();

	for (attrib, vals) in mesh.attributes() {
		let count = USize64(vals.len() as u64);
		let offset = bytes.len();
		let size = attrib.format.size();
		let attr_bytes = vals.get_bytes();
		bytes.extend_from_slice(attr_bytes);
		let view = PendingView {
			byte_length: USize64::from(attr_bytes.len()),
			byte_offset: Some(USize64::from(offset)),
			byte_stride: Some(Stride(size.try_into().unwrap())),
			name: Some(format!("{}_view", attrib.name)),
			target: Some(Valid(gltf_json::buffer::Target::ArrayBuffer)),
			extensions: None,
			extras: None,
		};

		let (min, max) = if attrib.id == Mesh::ATTRIBUTE_POSITION.id {
			vals.as_float3()
				.map(|vals| {
					let (min, max) =
						vals.iter()
							.fold((Vec3::MAX, Vec3::MIN), |(min, max), &val| {
								(
									Vec3::min(min, Vec3::from_array(val)),
									Vec3::max(max, Vec3::from_array(val)),
								)
							});
					(
						gltf_json::Value::from(Vec::from(min.to_array())),
						gltf_json::Value::from(Vec::from(max.to_array())),
					)
				})
				.unzip()
		} else {
			(None, None)
		};

		let accessor = PendingAccessor {
			count,
			component_type: Valid(GenericComponentType(attribute_component_type(
				attrib.format,
			))),
			extensions: None,
			extras: None,
			type_: Valid(attribute_type(attrib.format)),
			min,
			max,
			name: Some(attrib.name.into()),
			normalized: is_attribute_normalized(attrib.format),
			sparse: None,
		};
		attributes.insert(gltf_attribute_name(attrib), (view, accessor));
	}

	let (indices_view, indices_accessor) = if let Some(indices) = mesh.indices() {
		let indices_count = indices.len();
		let (indices_stride, indices_type, indices_bytes): (_, _, &[u8]) = match indices {
			Indices::U16(indices) => (
				Stride(size_of::<u16>()),
				GenericComponentType(gltf_json::accessor::ComponentType::U16),
				bytemuck::cast_slice(&indices),
			),
			Indices::U32(indices) => (
				Stride(size_of::<u32>()),
				GenericComponentType(gltf_json::accessor::ComponentType::U32),
				bytemuck::cast_slice(&indices),
			),
		};
		let offset = bytes.len();
		bytes.extend_from_slice(indices_bytes);
		let indices_view = PendingView {
			byte_length: USize64::from(indices_bytes.len()),
			byte_offset: Some(USize64::from(offset)),
			byte_stride: Some(indices_stride),
			name: Some("Indices_view".into()),
			target: None,
			extensions: None,
			extras: None,
		};
		let indices_accessor = PendingAccessor {
			count: USize64::from(indices_count),
			component_type: Valid(indices_type),
			extensions: None,
			extras: None,
			type_: Valid(gltf_json::accessor::Type::Scalar),
			min: None,
			max: None,
			name: Some("Indices".into()),
			normalized: false,
			sparse: None,
		};
		(Some(indices_view), Some(indices_accessor))
	} else {
		(None, None)
	};

	PendingAttributes {
		attributes,
		indices_view,
		indices_accessor,
	}
}

#[derive(Debug)]
struct PendingAttributes {
	attributes: BTreeMap<Checked<Semantic>, (PendingView, PendingAccessor)>,
	indices_view: Option<PendingView>,
	indices_accessor: Option<PendingAccessor>,
}

#[derive(Debug)]
struct PendingView {
	byte_length: USize64,
	byte_offset: Option<USize64>,
	byte_stride: Option<Stride>,
	name: Option<String>,
	target: Option<Checked<gltf_json::buffer::Target>>,
	extensions: Option<gltf_json::extensions::buffer::View>,
	extras: Extras,
}

#[derive(Debug)]
struct PendingAccessor {
	count: USize64,
	component_type: Checked<GenericComponentType>,
	extensions: Option<gltf_json::extensions::accessor::Accessor>,
	extras: Extras,
	type_: Checked<gltf_json::accessor::Type>,
	min: Option<gltf_json::Value>,
	max: Option<gltf_json::Value>,
	name: Option<String>,
	normalized: bool,
	sparse: Option<gltf_json::accessor::sparse::Sparse>,
}

fn push_scene(
	root: &mut gltf_json::Root,
	asset: &SavedAsset<Gltf>,
	id: AssetId<Scene>,
	deps: &HashMap<UntypedAssetId, &str>,
	mesh_ids: &HashMap<AssetId<GltfMesh>, Index<gltf_json::mesh::Mesh>>,
) -> Index<gltf_json::Scene> {
	if asset.scenes.len() > 1 {
		let _scene: &Scene = &*asset.get_labeled(deps[&id.untyped()]).unwrap();
		todo!("figure out how to associate nodes with specific scenes");
		// We can't query the Scene's world with only a shared reference.
		// (yet -- see https://github.com/bevyengine/bevy/issues/3774)
	}
	let mut root_nodes = asset.nodes.iter().collect::<Vec<_>>();
	for node in asset.nodes.iter() {
		let node: &GltfNode = &*asset.get_labeled(deps[&node.id().untyped()]).unwrap();
		root_nodes.retain(|maybe_root| !node.children.contains(*maybe_root));
	}
	let mut nodes = asset
		.nodes
		.iter()
		.map(|node| (node.id(), pending_node(asset, node.id(), deps, mesh_ids)))
		.collect::<HashMap<_, _>>();
	let mut node_order = Vec::new();
	let mut to_sort = nodes
		.iter()
		.map(|(id, node)| (*id, node.children.clone()))
		.collect::<BTreeMap<_, _>>();
	loop {
		if to_sort.is_empty() {
			break;
		}
		to_sort.retain(|id, children| {
			for child in children.iter().flatten() {
				if !node_order.contains(child) {
					return true;
				}
			}
			node_order.push(*id);
			false
		});
	}
	let mut node_indices = HashMap::new();
	let mut node_order = node_order.into_iter();
	loop {
		let Some(id) = node_order.next() else { break };
		let PendingNode {
			camera,
			children,
			extensions,
			extras,
			matrix,
			mesh,
			name,
			rotation,
			scale,
			translation,
			skin,
			weights,
		} = nodes.remove(&id).unwrap();
		let children = children.map(|children| {
			children
				.into_iter()
				.map(|child| node_indices[&child])
				.collect()
		});
		node_indices.insert(
			id,
			root.push(gltf_json::Node {
				camera,
				children,
				extensions,
				extras,
				matrix,
				mesh,
				name,
				rotation,
				scale,
				translation,
				skin,
				weights,
			}),
		);
	}
	let root_nodes = root_nodes
		.into_iter()
		.map(|handle| node_indices[&handle.id()])
		.collect();
	root.push(gltf_json::Scene {
		extensions: None,
		extras: None,
		name: None,
		nodes: root_nodes,
	})
}

fn pending_node(
	asset: &SavedAsset<Gltf>,
	id: AssetId<GltfNode>,
	deps: &HashMap<UntypedAssetId, &str>,
	mesh_ids: &HashMap<AssetId<GltfMesh>, Index<gltf_json::mesh::Mesh>>,
) -> PendingNode {
	let node: &GltfNode = &*asset.get_labeled(deps[&id.untyped()]).unwrap();
	let children = node.children.iter().map(Handle::id).collect::<Vec<_>>();
	PendingNode {
		camera: None, // TODO find cameras
		children: if children.len() > 0 {
			Some(children)
		} else {
			None
		},
		extensions: None,
		extras: gltf_json_extras(&node.extras),
		matrix: None, // Transform doesn't support skew anyway
		mesh: node.mesh.as_ref().map(|mesh| mesh_ids[&mesh.id()]),
		// TODO: Should we only include this for nodes in `named_nodes`?
		name: Some(node.name.clone()),
		rotation: Some(UnitQuaternion(node.transform.rotation.to_array())),
		scale: Some(node.transform.scale.to_array()),
		translation: Some(node.transform.translation.to_array()),
		skin: None,    // TODO
		weights: None, // TODO
	}
}

#[derive(Debug)]
pub struct PendingNode {
	camera: Option<Index<gltf_json::camera::Camera>>,
	children: Option<Vec<AssetId<GltfNode>>>,
	extensions: Option<gltf_json::extensions::scene::Node>,
	extras: Extras,
	matrix: Option<[f32; 16]>,
	mesh: Option<Index<gltf_json::mesh::Mesh>>,
	name: Option<String>,
	rotation: Option<UnitQuaternion>,
	scale: Option<[f32; 3]>,
	translation: Option<[f32; 3]>,
	skin: Option<Index<gltf_json::skin::Skin>>,
	weights: Option<Vec<f32>>,
}

#[derive(Debug, thiserror::Error)]
pub enum GlbSaverError {
	#[error(transparent)]
	Gltf(#[from] gltf::Error),
	#[error(transparent)]
	GltfJson(#[from] gltf_json::Error),
	#[error("buffer overflowed 4GiB")]
	BufferOversized(#[from] TryFromIntError),
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

// Utils

fn gltf_attribute_name(attrib: &MeshVertexAttribute) -> Checked<Semantic> {
	use gltf_json::mesh::Semantic::*;
	let id = attrib.id;
	Valid(if id == Mesh::ATTRIBUTE_POSITION.id {
		Positions
	} else if id == Mesh::ATTRIBUTE_NORMAL.id {
		Normals
	} else if id == Mesh::ATTRIBUTE_TANGENT.id {
		Tangents
	} else if id == Mesh::ATTRIBUTE_COLOR.id {
		Colors(0)
	} else if id == Mesh::ATTRIBUTE_UV_0.id {
		TexCoords(0)
	} else if id == Mesh::ATTRIBUTE_UV_1.id {
		TexCoords(1)
	} else if id == Mesh::ATTRIBUTE_JOINT_INDEX.id {
		Joints(0)
	} else if id == Mesh::ATTRIBUTE_JOINT_WEIGHT.id {
		Weights(0)
	} else {
		Extras(attrib.name.to_uppercase())
	})
}

fn is_attribute_normalized(format: VertexFormat) -> bool {
	use VertexFormat::*;
	match format {
		Unorm8x2 | Unorm8x4 | Snorm8x2 | Snorm8x4 | Unorm16x2 | Unorm16x4 | Snorm16x2
		| Snorm16x4 | Unorm10_10_10_2 => true,

		Uint8x2 | Uint8x4 | Sint8x2 | Sint8x4 | Uint16x2 | Uint16x4 | Sint16x2 | Sint16x4
		| Float16x2 | Float16x4 | Float32 | Float32x2 | Float32x3 | Float32x4 | Uint32
		| Uint32x2 | Uint32x3 | Uint32x4 | Sint32 | Sint32x2 | Sint32x3 | Sint32x4 | Float64
		| Float64x2 | Float64x3 | Float64x4 => false,
	}
}

fn attribute_component_type(format: VertexFormat) -> gltf_json::accessor::ComponentType {
	use VertexFormat::*;
	use gltf_json::accessor::ComponentType::*;
	match format {
		Uint8x2 | Uint8x4 | Unorm8x2 | Unorm8x4 => U8,

		Sint8x2 | Sint8x4 | Snorm8x2 | Snorm8x4 => I8,

		Uint16x2 | Uint16x4 | Unorm16x2 | Unorm16x4 => U16,

		Sint16x2 | Sint16x4 | Snorm16x2 | Snorm16x4 => I16,

		Float16x2 | Float16x4 => unimplemented!(
			"if Float16 is needed, we need to special case `get_bytes` and convert to F32"
		),

		Float32 | Float32x2 | Float32x3 | Float32x4 => F32,

		Uint32 | Uint32x2 | Uint32x3 | Uint32x4 | Unorm10_10_10_2 => U32,

		Sint32 | Sint32x2 | Sint32x3 | Sint32x4 => unimplemented!("gltf crate doesn't support I32"),

		Float64 | Float64x2 | Float64x3 | Float64x4 => {
			unimplemented!("gltf crate doesn't support F64")
		}
	}
}

fn attribute_type(format: VertexFormat) -> gltf_json::accessor::Type {
	use VertexFormat::*;
	use gltf_json::accessor::Type;
	match format {
		Float32 | Uint32 | Sint32 | Float64 => Type::Scalar,

		Uint8x2 | Sint8x2 | Unorm8x2 | Snorm8x2 | Uint16x2 | Sint16x2 | Unorm16x2 | Snorm16x2
		| Float16x2 | Float32x2 | Uint32x2 | Sint32x2 | Float64x2 => Type::Vec2,

		Float32x3 | Uint32x3 | Sint32x3 | Float64x3 => Type::Vec3,

		Uint8x4 | Sint8x4 | Unorm8x4 | Snorm8x4 | Uint16x4 | Sint16x4 | Unorm16x4 | Snorm16x4
		| Float16x4 | Float32x4 | Uint32x4 | Sint32x4 | Float64x4 => Type::Vec4,

		Unorm10_10_10_2 => unimplemented!("gltf crate doesn't support Unorm10_10_10_2"),
	}
}

pub fn gltf_json_extras(bevy_gltf_extras: &Option<GltfExtras>) -> Extras {
	bevy_gltf_extras
		.to_owned()
		.map(|extras| RawValue::from_string(extras.value).unwrap())
}

pub struct GltfZUpTransformer;

impl AssetTransformer for GltfZUpTransformer {
	type AssetInput = Gltf;
	type AssetOutput = Gltf;
	type Settings = ();
	type Error = Infallible;

	async fn transform<'a>(
		&'a self,
		mut asset: TransformedAsset<Self::AssetInput>,
		_settings: &'a Self::Settings,
	) -> Result<TransformedAsset<Self::AssetOutput>, Self::Error> {
		let deps = asset
			.iter_labels()
			.map(|label| {
				(
					asset.get_untyped_handle(label).unwrap().id(),
					label.to_owned(),
				)
			})
			.collect::<HashMap<UntypedAssetId, String>>();

		let gltf_mesh_ids = asset.meshes.iter().map(Handle::id).collect::<Vec<_>>();
		let mut mesh_ids = Vec::new();
		for gltf_mesh in gltf_mesh_ids {
			let gltf_mesh = asset
				.get_labeled::<GltfMesh, _>(&*deps[&gltf_mesh.untyped()])
				.unwrap();
			for prim in gltf_mesh.primitives.iter() {
				mesh_ids.push(prim.mesh.id())
			}
		}
		for mesh in mesh_ids {
			let mut mesh = asset
				.get_labeled::<Mesh, _>(&*deps[&mesh.untyped()])
				.unwrap();
			mesh.make_z_up();
		}

		let node_ids = asset.nodes.iter().map(Handle::id).collect::<Vec<_>>();
		for node in node_ids {
			let mut node = asset
				.get_labeled::<GltfNode, _>(&*deps[&node.untyped()])
				.unwrap();
			node.transform.translation.make_z_up();
		}

		drop(deps); // Drop check thinks assets is still borrowed
		Ok(asset)
	}
}
