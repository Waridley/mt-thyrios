use crate::game::mtn::terrain::{
	StackedTerrainTextures, TerrainKind, TexturesMap, stack_terrain_textures,
};
use crate::game::{
	AssetsLoaded, GameLoadingState, GameSetupKey, GameSetupLabel,
	mtn::terrain::TerrainTexturesStacked,
};
use crate::new_game_setup_label;
use crate::setup_tracking::{IntoDependencyProvider, RegisterProvider, single_spawn_progress};
use crate::state::GlobalState;
use crate::util::{FinishedProcessing, GridMesh, MeshExt};
use GlobalState::{InGame, LoadingGame};
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::basic::FUCHSIA;
use bevy::ecs::system::IntoObserverSystem;
use bevy::image::{ImageAddressMode, ImageLoaderSettings};
use bevy::math::Vec3A;
use bevy::math::bounding::{
	Aabb3d, Bounded3d, BoundingSphere, BoundingVolume, IntersectsVolume, RayCast3d,
};
use bevy::pbr::wireframe::Wireframe;
use bevy::pbr::{
	ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::picking::mesh_picking::ray_cast::ray_aabb_intersection_3d;
use bevy::prelude::*;
use bevy::render::mesh::skinning::SkinnedMesh;
use bevy::render::mesh::{
	Indices, MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexAttributeValues,
};
use bevy::render::render_resource::{
	AsBindGroup, Extent3d, Face, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
	VertexFormat,
};
use bevy::scene::SceneInstance;
use bevy::utils::HashSet;
use bevy_asset_loader::prelude::AssetCollection;
use smolset::SmolSet;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::Index;
use terrain::{ATTRIBUTE_TERRAIN_WEIGHTS_0_3, ATTRIBUTE_TERRAIN_WEIGHTS_4_7, TerrainMaterial};
use tiny_bail::prelude::r;

pub mod terrain;

pub const PEAK_ELEVATION: f32 = 75.0;
pub const RADIUS_TOP: f32 = 10.0;
pub const SLOPE: f32 = 0.5;
pub const BOTTOM_DEPTH: f32 = 15.0;
pub const TOTAL_HEIGHT: f32 = PEAK_ELEVATION + BOTTOM_DEPTH;

new_game_setup_label!(
	MountainSceneSpawned,
	single_spawn_progress::<(With<MountainScene>, With<SceneInstance>)>
);
new_game_setup_label!(MountainHydrated, single_spawn_progress::<With<Mountain>>);

pub struct MountainPlugin;

impl Plugin for MountainPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaterialPlugin::<
			ExtendedMaterial<StandardMaterial, TerrainMaterial>,
		>::default())
			.register_type::<TerrainKind>()
			.add_systems(OnEnter(LoadingGame), |mut cmds: Commands| {
				cmds.init_resource::<StackedTerrainTextures>()
			})
			.register_provider(
				spawn_mountain
					.provides([MountainSceneSpawned.intern()])
					.requires([AssetsLoaded.intern()]),
			)
			.register_provider(
				stack_terrain_textures
					.provides([TerrainTexturesStacked.intern()])
					.requires([AssetsLoaded.intern()]),
			)
			.register_provider(
				setup_mountain
					.provides([MountainHydrated.intern()])
					.requires([MountainSceneSpawned.intern()]),
			)
			.add_systems(OnExit(InGame), |mut cmds: Commands| {
				cmds.remove_resource::<MountainAssets>();
				cmds.remove_resource::<TexturesMap>();
				cmds.remove_resource::<StackedTerrainTextures>();
			});
	}
}

#[derive(AssetCollection, Resource, Debug)]
pub struct MountainAssets {
	#[asset(path = "terrain/mtn.glb#Scene0")]
	pub mountain_scene: Handle<Scene>,
	#[asset(path = "dbg_grid.png")]
	#[asset(image(array_texture_layers = 6, sampler(wrap = repeat)))]
	pub terrain_textures: Handle<Image>,
}

pub fn spawn_mountain(
	mut cmds: Commands,
	assets: Res<MountainAssets>,
	existing_scene: Option<Single<Entity, With<MountainScene>>>,
) {
	if let Some(_) = existing_scene {
		return;
	}
	// let mut mtn = Mountain {
	// 	peak_elevation: PEAK_ELEVATION,
	// 	plateau_radius: RADIUS_TOP,
	// 	slope: SLOPE,
	// 	bottom_depth: BOTTOM_DEPTH,
	// };
	//
	// let mut mesh = mtn.grid_mesh().subdivisions(512).build();
	//
	// mesh.asset_usage = RenderAssetUsages::all();
	// let positions = mesh.positions().unwrap();
	//
	// let len = positions.len();
	// let tris = mesh.indices().unwrap().len() / 3;
	// debug!("Mountain triangles: {tris} ({len} vertices)");
	// let weights = vec![[255u16, 0, 0, 0]; len];
	// mesh.insert_attribute(
	// 	ATTRIBUTE_TERRAIN_WEIGHTS_0_3,
	// 	VertexAttributeValues::Unorm16x4(weights),
	// );
	// let weights = vec![[0u16; 4]; len];
	// mesh.insert_attribute(
	// 	ATTRIBUTE_TERRAIN_WEIGHTS_4_7,
	// 	VertexAttributeValues::Unorm16x4(weights),
	// );
	//
	// let graph = MeshGraph::build(&mesh).unwrap();

	cmds.spawn((
		MountainScene,
		// mtn,
		// Mesh3d(meshes.add(mesh)),
		SceneRoot(assets.mountain_scene.clone()),
		// MeshMaterial3d::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>(mats.add(
		// 	ExtendedMaterial {
		// 		extension: TerrainMaterial {
		// 			textures: assets.terrain_textures.clone(),
		// 		},
		// 		base: StandardMaterial {
		// 			reflectance: 0.25,
		// 			perceptual_roughness: 0.9,
		// 			..default()
		// 		},
		// 	},
		// )),
		// graph,
		StateScoped(InGame),
	))
	// .with_child((
	// 	MountainPeak,
	// 	Transform::from_translation(Vec3::Z * PEAK_ELEVATION),
	// ))
	;
}

pub fn setup_mountain(
	mut cmds: Commands,
	assets: Res<MountainAssets>,
	existing_mtn: Query<Entity, With<Mountain>>,
	existing_peak: Query<Entity, With<MountainPeak>>,
	names: Query<(Entity, &Name), Added<Name>>,
	mut mats: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mesh_handles: Query<&Mesh3d>,
	mut next_state: ResMut<NextState<GameLoadingState>>,
) {
	if !existing_mtn.is_empty() && !existing_peak.is_empty() {
		next_state.set(GameLoadingState::SceneBuilt);
		return;
	}
	for (id, name) in names.iter() {
		if *name == Name::new("MountainPeak") {
			debug!("MountainPeak: {id:?}");
			cmds.entity(id).insert(MountainPeak);
		}
		if *name == Name::new("MountainMeshMesh") {
			debug!("MountainMesh: {id:?}");
			let mut mtn = Mountain {
				peak_elevation: PEAK_ELEVATION,
				plateau_radius: RADIUS_TOP,
				slope: SLOPE,
				bottom_depth: BOTTOM_DEPTH,
			};
			let mesh = r!(mesh_handles.get(id));
			let mesh = r!(meshes.get_mut(&mesh.0));
			let graph = MeshGraph::build(&mesh).unwrap();
			cmds.entity(id)
				.remove::<MeshMaterial3d<StandardMaterial>>()
				.insert((
					mtn,
					MeshMaterial3d::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>(
						mats.add(ExtendedMaterial {
							extension: TerrainMaterial {
								textures: assets.terrain_textures.clone(),
							},
							base: StandardMaterial {
								reflectance: 0.25,
								perceptual_roughness: 0.9,
								..default()
							},
						}),
					),
					graph,
				));
		}
	}
}

#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility, StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct Mountain {
	pub peak_elevation: f32,
	pub plateau_radius: f32,
	pub slope: f32,
	pub bottom_depth: f32,
}

#[derive(Component, Debug)]
pub struct MountainScene;

impl Mountain {
	pub const fn total_height(&self) -> f32 {
		self.peak_elevation + self.bottom_depth
	}

	pub const fn bottom_radius(&self) -> f32 {
		self.plateau_radius + (self.total_height() / self.slope)
	}
}

#[derive(Component, Debug)]
#[require(Transform, Visibility)]
pub struct MountainPeak;

#[derive(Component, Debug, Default, Clone)]
pub struct MeshGraph {
	pub triangles_using_vertices: Vec<SmolSet<[TriangleIndex; 6]>>,
	pub vertices_adjacent_to_vertices: Vec<SmolSet<[VertexIndex; 6]>>,
	pub bvh: Bvh<TriangleIndex>,
}

pub type VertexIndex = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriangleIndex(usize);

impl TriangleIndex {
	pub fn vertex_indices(&self, indices: &Indices) -> [VertexIndex; 3] {
		let i = self.0 * 3;
		match indices {
			Indices::U16(indices) => [
				indices[i] as usize,
				indices[i + 1] as usize,
				indices[i + 2] as usize,
			],
			Indices::U32(indices) => [
				indices[i] as usize,
				indices[i + 1] as usize,
				indices[i + 2] as usize,
			],
		}
	}

	pub fn triangle(&self, mesh: &Mesh) -> Triangle3d {
		let indices = self.vertex_indices(mesh.indices().unwrap());
		let positions = mesh.positions().unwrap();
		Triangle3d {
			vertices: [
				positions[indices[0]].into(),
				positions[indices[1]].into(),
				positions[indices[2]].into(),
			],
		}
	}

	pub fn iter(mesh_indices: &Indices) -> impl Iterator<Item = TriangleIndex> + use<'_> {
		mesh_indices.iter().enumerate().step_by(3).map(|(i, _)| {
			debug_assert_eq!(i % 3, 0);
			Self(i / 3)
		})
	}
}

impl MeshGraph {
	pub fn build(mesh: &Mesh) -> Option<Self> {
		let len = mesh.positions()?.len();
		let mut tris = vec![SmolSet::new(); len];
		let mut adj = vec![SmolSet::new(); len];

		if let Some(indices) = mesh.indices() {
			debug_assert!(indices.len() % 3 == 0);
			for (i, [a, b, c]) in indices.iter().array_chunks::<3>().enumerate() {
				let i = TriangleIndex(i);
				tris[a].insert(i);
				adj[a].insert(b);
				adj[a].insert(c);

				tris[b].insert(i);
				adj[b].insert(a);
				adj[b].insert(c);

				tris[c].insert(i);
				adj[c].insert(a);
				adj[c].insert(b);
			}
		} else {
			debug_assert!(len % 3 == 0);
			for i in 0..(len / 3) {
				let ia = i * 3;
				let ib = ia + 1;
				let ic = ia + 2;
				tris[ia].insert(TriangleIndex(i));
				adj[ia].insert(ib);
				adj[ia].insert(ic);

				tris[ib].insert(TriangleIndex(i));
				adj[ib].insert(ia);
				adj[ib].insert(ic);

				tris[ic].insert(TriangleIndex(i));
				adj[ic].insert(ia);
				adj[ic].insert(ib);
			}
		}

		Some(Self {
			triangles_using_vertices: tris,
			vertices_adjacent_to_vertices: adj,
			bvh: Bvh::build(mesh),
		})
	}

	pub fn triangles_using(&self, i: VertexIndex) -> &SmolSet<[TriangleIndex; 6]> {
		&self.triangles_using_vertices[i]
	}

	pub fn adjacency_list(&self, i: VertexIndex) -> &SmolSet<[VertexIndex; 6]> {
		&self.vertices_adjacent_to_vertices[i]
	}
}

pub trait Ray3dExt {
	/// Iterates through all vertices and selects the one with the minimum distance from the ray.
	/// Slower than [Ray3dExt::cached_closest_vertex] but is immune to local minima.
	fn absolute_closest_vertex(self, mesh: &Mesh) -> Option<VertexIndex>;

	/// Uses [MeshGraph] to reach the closest vertex more quickly. Can get stuck in a local minimum.
	/// If this ray is close to the previous ray, it is unlikely that a new local minimum will be near
	/// enough to fall into.
	fn cached_closest_vertex(
		self,
		mesh: &Mesh,
		graph: &MeshGraph,
		previous_closest: VertexIndex,
	) -> VertexIndex;

	/// The squared distance from `point` to this ray.
	fn distance_squared_from_point(self, point: Vec3) -> f32;

	fn find_intersected_triangle(
		self,
		mesh: &Mesh,
		graph: &MeshGraph,
		cull_mode: Option<Face>,
	) -> Option<(TriangleIndex, f32)>;

	fn intersect_triangle(self, triangle: Triangle3d, cull_mode: Option<Face>) -> Option<f32>;
}

impl Ray3dExt for Ray3d {
	fn absolute_closest_vertex(self, mesh: &Mesh) -> Option<VertexIndex> {
		mesh.iter_positions()
			.copied()
			.map(Vec3::from_array)
			.enumerate()
			.map(|(i, pos)| (i, self.distance_squared_from_point(pos)))
			.min_by(|(_, da), (_, db)| da.partial_cmp(&db).unwrap_or(Ordering::Equal))
			.map(|(i, _)| i)
	}

	fn cached_closest_vertex(
		self,
		mesh: &Mesh,
		graph: &MeshGraph,
		previous_closest: VertexIndex,
	) -> VertexIndex {
		let positions = mesh.positions().unwrap();
		let mut dist =
			self.distance_squared_from_point(Vec3::from_array(positions[previous_closest]));
		let mut closest = previous_closest;
		loop {
			let adj = &graph.adjacency_list(previous_closest);
			let next_closest = adj
				.iter()
				.copied()
				.map(|other| (other, Vec3::from_array(positions[other])))
				.map(|(i, pos)| (i, self.distance_squared_from_point(pos)))
				.min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal));
			if let Some((i, d)) = next_closest {
				if d < dist {
					closest = i;
					dist = d;
				} else {
					return closest;
				}
			} else {
				return closest;
			}
		}
	}

	fn distance_squared_from_point(self, point: Vec3) -> f32 {
		let pos = point - self.origin;
		let projection = pos.dot(*self.direction) * self.direction;
		pos.distance_squared(projection)
	}

	fn find_intersected_triangle(
		self,
		mesh: &Mesh,
		graph: &MeshGraph,
		cull_mode: Option<Face>,
	) -> Option<(TriangleIndex, f32)> {
		find_triangle_intersected_by_ray(self, mesh, graph, cull_mode)
	}

	fn intersect_triangle(self, triangle: Triangle3d, cull_mode: Option<Face>) -> Option<f32> {
		let Triangle3d {
			vertices: [a, b, c],
		} = triangle;

		// https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection.html
		let ab = b - a;
		let ac = c - a;
		let p_vec = self.direction.cross(ac);
		let det = ab.dot(p_vec);

		cull_by_dot_product(det, cull_mode)?;

		let inv_det = 1.0 / det;

		let t_vec = self.origin - a;
		let u = t_vec.dot(p_vec) * inv_det;
		if u < 0.0 || u > 1.0 {
			return None;
		}

		let q_vec = t_vec.cross(ab);
		let v = self.direction.dot(q_vec) * inv_det;
		if v < 0.0 || u + v > 1.0 {
			return None;
		}

		Some(ac.dot(q_vec) * inv_det)
	}
}

#[derive(Component, Clone, Debug)]
pub struct Bvh<T> {
	pub root: BvhNode<T>,
}

impl<T> Default for Bvh<T> {
	fn default() -> Self {
		Self { root: default() }
	}
}

#[derive(Debug, Clone)]
pub enum BvhContents<T> {
	Branch(Box<[BvhNode<T>]>),
	Leaf(Vec<TriangleIndex>),
}

#[derive(Debug, Clone)]
pub struct BvhNode<T> {
	pub aabb: Aabb3d,
	pub contents: BvhContents<T>,
	pub _marker: PhantomData<T>,
}

impl BvhNode<TriIdx> {
	pub fn all_triangles(&self) -> impl Iterator<Item = TriIdx> + use<'_> {
		AllBvhTriangles {
			stack: vec![self],
			iter: [].iter(),
		}
	}
}

struct AllBvhTriangles<'a> {
	stack: Vec<&'a BvhNode<TriIdx>>,
	iter: std::slice::Iter<'a, TriIdx>,
}

impl Iterator for AllBvhTriangles<'_> {
	type Item = TriIdx;

	fn next(&mut self) -> Option<Self::Item> {
		let Self { stack, iter } = self;
		loop {
			if let Some(idx) = iter.next() {
				return Some(*idx);
			} else {
				match stack.pop() {
					Some(node) => match &node.contents {
						BvhContents::Branch(subtrees) => {
							for subtree in subtrees {
								stack.push(subtree);
							}
						}
						BvhContents::Leaf(tris) => *iter = tris.iter(),
					},
					None => return None,
				}
			}
		}
	}
}

const INVALID_AABB: Aabb3d = Aabb3d {
	min: Vec3A::MAX,
	max: Vec3A::MIN,
};

impl<T> Default for BvhNode<T> {
	fn default() -> Self {
		Self {
			aabb: INVALID_AABB,
			contents: BvhContents::<T>::Leaf(Vec::new()),
			_marker: PhantomData,
		}
	}
}

type TriIdx = TriangleIndex;

impl Bvh<TriIdx> {
	pub fn build(mesh: &Mesh) -> Self {
		let mut full_aabb = INVALID_AABB;
		let triangles = mesh
			.indices()
			.map(TriIdx::iter)
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		for &tri in &triangles {
			let tri = tri.triangle(mesh);
			let aabb = tri.aabb_3d(Isometry3d::default());
			full_aabb = full_aabb.merge(&aabb);
		}

		const MAX_DEPTH: usize = 16;
		const FEW_ENOUGH_TRIS: usize = 256;
		fn split(
			mesh: &Mesh,
			aabb: Aabb3d,
			triangles: Vec<TriIdx>,
			depth: usize,
		) -> BvhNode<TriIdx> {
			if depth >= MAX_DEPTH || triangles.len() <= FEW_ENOUGH_TRIS || triangles.is_empty() {
				return BvhNode {
					aabb,
					contents: BvhContents::Leaf(triangles),
					_marker: PhantomData,
				};
			}

			let center = aabb.center();
			let mut l_aabb = INVALID_AABB;
			let mut lesser = Vec::new();
			let mut g_aabb = INVALID_AABB;
			let mut greater = Vec::new();
			for tri_idx in triangles {
				let tri = tri_idx.triangle(mesh);
				let tri_center = tri.centroid();
				let axis = aabb
					.half_size()
					.to_array()
					.into_iter()
					.enumerate()
					.max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(Ordering::Equal))
					.map(|(i, _)| i)
					.unwrap_or(0);
				if tri_center[axis] <= center[axis] {
					l_aabb = l_aabb.merge(&tri.aabb_3d(Isometry3d::default()));
					lesser.push(tri_idx);
				} else {
					g_aabb = g_aabb.merge(&tri.aabb_3d(Isometry3d::default()));
					greater.push(tri_idx);
				}
			}
			let subtrees = [(l_aabb, lesser), (g_aabb, greater)]
				.into_iter()
				.map(|(aabb, triangles)| split(mesh, aabb, triangles, depth + 1))
				.collect();
			BvhNode {
				aabb,
				contents: BvhContents::Branch(subtrees),
				_marker: PhantomData,
			}
		}
		let root = split(mesh, full_aabb, triangles, 0);

		Self { root }
	}
}

fn cull_by_dot_product(dot: f32, cull_mode: Option<Face>) -> Option<()> {
	if dot <= f32::EPSILON && dot >= -f32::EPSILON
		|| matches!(cull_mode, None | Some(Face::Back)) && dot < -f32::EPSILON
		|| matches!(cull_mode, None | Some(Face::Front)) && dot > f32::EPSILON
	{
		None
	} else {
		Some(())
	}
}

#[inline(always)]
fn no_dbg_aabb(_aabb: &BvhNode<TriIdx>, _depth: usize) {}

#[inline(always)]
fn no_dbg_tris(_tris: &HashSet<TriIdx>) {}

#[inline]
pub fn find_triangle_intersected_by_ray(
	ray: Ray3d,
	mesh: &Mesh,
	graph: &MeshGraph,
	cull_mode: Option<Face>,
) -> Option<(TriIdx, f32)> {
	debug_find_triangle_intersected_by_ray(ray, mesh, graph, cull_mode, no_dbg_aabb, no_dbg_tris)
}

pub fn debug_find_triangle_intersected_by_ray(
	ray: Ray3d,
	mesh: &Mesh,
	graph: &MeshGraph,
	cull_mode: Option<Face>,
	mut dbg_aabb: impl FnMut(&BvhNode<TriIdx>, usize),
	mut dbg_tris: impl FnOnce(&HashSet<TriIdx>),
) -> Option<(TriIdx, f32)> {
	let positions = mesh.positions().unwrap();

	let mut tris = HashSet::new();
	fn collect_tris(
		ray: Ray3d,
		node: &BvhNode<TriIdx>,
		tris: &mut HashSet<TriIdx>,
		depth: usize,
		dbg_fn: &mut impl FnMut(&BvhNode<TriIdx>, usize),
	) {
		if RayCast3d::from_ray(ray, crate::game::ocean::RADIUS * 2.0).intersects(&node.aabb) {
			dbg_fn(node, depth);
			match &node.contents {
				BvhContents::Branch(subtrees) => {
					for subtree in subtrees.iter() {
						collect_tris(ray, subtree, tris, depth + 1, dbg_fn);
					}
				}
				BvhContents::Leaf(leaf) => tris.extend(leaf.iter().copied()),
			}
		}
	}
	collect_tris(ray, &graph.bvh.root, &mut tris, 0, &mut dbg_aabb);
	dbg_tris(&tris);

	for idx in tris {
		let [a, b, c] = idx.vertex_indices(mesh.indices().unwrap());
		let tri = Triangle3d::new(
			Vec3::from_array(positions[a]),
			Vec3::from_array(positions[b]),
			Vec3::from_array(positions[c]),
		);
		let intersection = ray.intersect_triangle(tri, cull_mode);
		if let Some(t) = intersection {
			return Some((idx, t));
		}
	}
	None
}
