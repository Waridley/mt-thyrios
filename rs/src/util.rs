use crate::game::mtn::Mountain;
use bevy::ecs::query::{QueryFilter, QuerySingleError};
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::{ScheduleSystem, SystemParam};
use bevy::gltf::{GltfError, GltfMesh};
use bevy::math::Vec3A;
use bevy::math::bounding::Aabb3d;
use bevy::render::mesh::{Indices, VertexAttributeValues};
use bevy::state::state::FreelyMutableState;
use bevy::{
	asset::RenderAssetUsages, math::primitives::Plane3d, prelude::*,
	render::mesh::PrimitiveTopology,
};
use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::error::Error;

pub mod gltf;

pub struct CircleGridMeshBuilder<T: Vertex3Difier = ()> {
	pub circle: Circle,
	pub vert_xform: T,
	pub subdivisions: u32,
}

impl Default for CircleGridMeshBuilder {
	fn default() -> Self {
		Self {
			circle: default(),
			vert_xform: (),
			subdivisions: 0,
		}
	}
}

impl<F: Vertex3Difier> CircleGridMeshBuilder<F> {
	pub fn subdivisions(mut self, subdivisions: u32) -> Self {
		self.subdivisions = subdivisions;
		self
	}

	pub fn map_vertices<NewF: Vertex3Difier>(mut self, f: NewF) -> CircleGridMeshBuilder<NewF> {
		CircleGridMeshBuilder {
			circle: self.circle,
			vert_xform: f,
			subdivisions: self.subdivisions,
		}
	}
}

impl<F: Vertex3Difier> MeshBuilder for CircleGridMeshBuilder<F> {
	fn build(&self) -> Mesh {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::default(),
		);
		let mut verts = Vec::with_capacity((self.subdivisions as usize + 2) * 4);
		let mut uvs = Vec::with_capacity((self.subdivisions as usize + 2) * 4);
		let x_verts = self.subdivisions + 2;
		let y_verts = self.subdivisions + 2;
		let cap = ((x_verts as f64 * y_verts as f64) * std::f64::consts::FRAC_PI_4).ceil() as usize;
		let mut indices = if x_verts < 288 {
			Indices::U16(Vec::with_capacity(cap))
		} else {
			Indices::U32(Vec::with_capacity(cap))
		};
		let mut grid = vec![vec![None; x_verts as usize]; y_verts as usize];
		let slices = self.subdivisions + 1;
		let r = self.circle.radius;
		let d = r * 2.0;
		for y in 0..y_verts {
			for x in 0..x_verts {
				let mut cell: &mut Option<u32> = &mut grid[y as usize][x as usize];
				let y = ((y as f32 * d) / slices as f32) - r;
				let x = ((x as f32 * d) / slices as f32) - r;
				if Vec2::new(x, y).length_squared() <= r * r {
					*cell = Some(verts.len() as u32);
					let v = Vec2::new(x, y);
					let pos = self.vert_xform.to_3d(v);
					verts.push(pos);
					uvs.push(Vec2::new((x / d) + 0.5, (y / d) + 0.5));
				}
			}
		}
		use Ordering::*;
		enum Dir {
			Left,
			Right,
			Both,
		}
		for y in 0..slices {
			for x in 0..x_verts {
				if let Some(ia) = grid[y as usize][x as usize] {
					let dir = match (y.cmp(&(y_verts / 2)), x.cmp(&(x_verts / 2))) {
						// bottom-left | top-right
						(Less, Less) | (Greater, Greater) | (Equal, Greater) => Dir::Left,
						// bottom-right | top-left
						(Less, Greater) | (Greater, Less) | (Equal, Less) => Dir::Right,
						// x direction is outward for -y but inward for +y
						(Less, Equal) => Dir::Both,
						(Greater, Equal) | (Equal, Equal) => continue,
					};
					let (y, x) = (y as usize, x as usize);
					let tris: &[((isize, isize), (isize, isize))] = match dir {
						Dir::Left => {
							if x == 0 {
								continue;
							} else {
								&[((1, -1), (0, -1)), ((1, 0), (1, -1))]
							}
						}
						Dir::Right => {
							if x >= (x_verts - 1) as usize {
								continue;
							} else {
								&[((1, 1), (1, 0)), ((0, 1), (1, 1))]
							}
						}
						Dir::Both => &[
							((1, -1), (0, -1)),
							((1, 0), (1, -1)),
							((1, 1), (1, 0)),
							((0, 1), (1, 1)),
						],
					};
					for &((by, bx), (cy, cx)) in tris.iter() {
						let by = y.checked_add_signed(by).unwrap();
						let bx = x.checked_add_signed(bx).unwrap();
						let cy = y.checked_add_signed(cy).unwrap();
						let cx = x.checked_add_signed(cx).unwrap();
						if let Some((ib, ic)) = grid[by][bx].zip(grid[cy][cx]) {
							indices.push(ia);
							indices.push(ib);
							indices.push(ic);
						}
					}
				}
			}
		}
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verts);
		// mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
		mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
		mesh.insert_indices(indices);
		mesh.compute_normals();
		mesh
	}
}

/// Trait for subdividing a mesh as a grid instead of the default `Meshable` implementation for a shape.
pub trait GridMesh {
	type Builder: MeshBuilder;
	fn grid_mesh(&self) -> Self::Builder;
}

impl GridMesh for Plane3d {
	type Builder = <Plane3d as Meshable>::Output;

	fn grid_mesh(&self) -> Self::Builder {
		self.mesh()
	}
}

impl GridMesh for Circle {
	type Builder = CircleGridMeshBuilder;

	fn grid_mesh(&self) -> Self::Builder {
		CircleGridMeshBuilder {
			circle: *self,
			..default()
		}
	}
}

impl GridMesh for Mountain {
	type Builder = CircleGridMeshBuilder<Self>;

	fn grid_mesh(&self) -> Self::Builder {
		CircleGridMeshBuilder {
			circle: Circle::new(self.bottom_radius()),
			vert_xform: self.clone(),
			subdivisions: (self.bottom_radius() / self.plateau_radius) as _,
		}
	}
}

pub trait Vertex3Difier {
	fn to_3d(&self, v: Vec2) -> Vec3;
}

impl Vertex3Difier for () {
	fn to_3d(&self, v: Vec2) -> Vec3 {
		Vec3::new(v.x, 0.0, -v.y)
	}
}

impl Vertex3Difier for Mountain {
	fn to_3d(&self, v: Vec2) -> Vec3 {
		let len = v.length();
		if len <= self.plateau_radius {
			Vec3::new(v.x, v.y, self.peak_elevation)
		} else if len > self.bottom_radius() {
			Vec3::new(v.x, v.y, self.bottom_depth)
		} else {
			let slope = self.slope;
			Vec3::new(
				v.x,
				v.y,
				(slope * (self.bottom_radius() - len)) - self.bottom_depth,
			)
		}
	}
}

impl<F: Fn(Vec2) -> Vec3> Vertex3Difier for F {
	fn to_3d(&self, v: Vec2) -> Vec3 {
		self(v)
	}
}

pub trait MeshExt {
	fn positions(&self) -> Option<&Vec<[f32; 3]>>;
	fn positions_mut(&mut self) -> Option<&mut Vec<[f32; 3]>>;
	fn iter_positions(&self) -> impl Iterator<Item = &[f32; 3]>;
	fn iter_positions_mut(&mut self) -> impl Iterator<Item = &mut [f32; 3]>;
	fn normals(&self) -> Option<&Vec<[f32; 3]>>;
	fn normals_mut(&mut self) -> Option<&mut Vec<[f32; 3]>>;
	fn iter_normals(&self) -> impl Iterator<Item = &[f32; 3]>;
	fn iter_normals_mut(&mut self) -> impl Iterator<Item = &mut [f32; 3]>;
}

impl MeshExt for Mesh {
	#[inline]
	fn positions(&self) -> Option<&Vec<[f32; 3]>> {
		self.attribute(Self::ATTRIBUTE_POSITION)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values,
				_ => unreachable!(),
			})
	}

	#[inline]
	fn positions_mut(&mut self) -> Option<&mut Vec<[f32; 3]>> {
		self.attribute_mut(Self::ATTRIBUTE_POSITION)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values,
				_ => unreachable!(),
			})
	}

	#[inline]
	fn iter_positions(&self) -> impl Iterator<Item = &[f32; 3]> {
		self.attribute(Self::ATTRIBUTE_POSITION)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values.iter(),
				_ => unreachable!(),
			})
			.into_iter()
			.flatten()
	}

	#[inline]
	fn iter_positions_mut(&mut self) -> impl Iterator<Item = &mut [f32; 3]> {
		self.attribute_mut(Self::ATTRIBUTE_POSITION)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values.iter_mut(),
				_ => unreachable!(),
			})
			.into_iter()
			.flatten()
	}

	fn normals(&self) -> Option<&Vec<[f32; 3]>> {
		self.attribute(Self::ATTRIBUTE_NORMAL)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values,
				_ => unreachable!(),
			})
	}

	fn normals_mut(&mut self) -> Option<&mut Vec<[f32; 3]>> {
		self.attribute_mut(Self::ATTRIBUTE_NORMAL)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values,
				_ => unreachable!(),
			})
	}

	fn iter_normals(&self) -> impl Iterator<Item = &[f32; 3]> {
		self.attribute(Self::ATTRIBUTE_NORMAL)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values.iter(),
				_ => unreachable!(),
			})
			.into_iter()
			.flatten()
	}

	fn iter_normals_mut(&mut self) -> impl Iterator<Item = &mut [f32; 3]> {
		self.attribute_mut(Self::ATTRIBUTE_NORMAL)
			.map(|vals| match vals {
				VertexAttributeValues::Float32x3(values) => values.iter_mut(),
				_ => unreachable!(),
			})
			.into_iter()
			.flatten()
	}
}

pub fn log_errors<T, E: Error>(e: In<Result<T, E>>) -> Result<T, E> {
	if let Err(e) = &e.0 {
		error!("{}", e);
	}
	e.0
}

pub trait ZUp {
	fn make_z_up(&mut self);
}

impl ZUp for Vec3 {
	fn make_z_up(&mut self) {
		let new_z = self.y;
		self.y = -self.z;
		self.z = new_z;
	}
}

impl ZUp for [f32; 3] {
	fn make_z_up(&mut self) {
		let new_z = self[1];
		self[1] = -self[2];
		self[2] = new_z;
	}
}

impl<T: ZUp> ZUp for [T] {
	fn make_z_up(&mut self) {
		for val in self {
			val.make_z_up();
		}
	}
}

impl ZUp for Mesh {
	fn make_z_up(&mut self) {
		for (_, vals) in self.attributes_mut() {
			if let VertexAttributeValues::Float32x3(vals) = vals {
				vals.make_z_up()
			}
		}
	}
}

#[derive(SystemParam)]
pub struct GltfSubAssets<'w> {
	pub scenes: ResMut<'w, Assets<Scene>>,
	pub gltf_meshes: ResMut<'w, Assets<GltfMesh>>,
	pub meshes: ResMut<'w, Assets<Mesh>>,
}

pub struct GltfProcessor<'a, 'w: 'a> {
	gltf: &'a Gltf,
	assets: &'a mut GltfSubAssets<'w>,
}

impl<'a, 'w: 'a> GltfProcessor<'a, 'w> {
	pub fn process_meshes(&mut self, mut f: impl FnMut(Mut<Mesh>)) {
		let GltfSubAssets {
			gltf_meshes,
			meshes,
			..
		} = self.assets;
		for mesh in self.gltf.meshes.iter() {
			let mesh = gltf_meshes
				.reborrow()
				.map_unchanged(|meshes| meshes.get_mut(mesh).unwrap());
			for mesh in &mesh.primitives {
				let mesh = meshes
					.reborrow()
					.map_unchanged(|meshes| meshes.get_mut(&mesh.mesh).unwrap());
				f(mesh)
			}
		}
	}
	pub fn process_scenes(&mut self, mut f: impl FnMut(Mut<Scene>)) {
		for scene in self.gltf.scenes.iter() {
			let scene = self
				.assets
				.scenes
				.reborrow()
				.map_unchanged(|scenes| scenes.get_mut(scene).unwrap());
			f(scene);
		}
	}
}

pub trait ProcessGltf {
	fn process<'a, 'w: 'a>(&'a self, assets: &'a mut GltfSubAssets<'w>) -> GltfProcessor<'a, 'w>;
}

impl ProcessGltf for Gltf {
	fn process<'a, 'w: 'a>(&'a self, assets: &'a mut GltfSubAssets<'w>) -> GltfProcessor<'a, 'w> {
		GltfProcessor { gltf: self, assets }
	}
}

impl ZUp for GltfProcessor<'_, '_> {
	fn make_z_up(&mut self) {
		self.process_meshes(|mut mesh| mesh.make_z_up());
		self.process_scenes(|mut scene| {
			let mut q = scene.world.query::<&mut Transform>();
			for mut xform in q.iter_mut(&mut scene.world) {
				xform.translation.make_z_up();

				// Rotation was just applied to mesh vertices directly. If any components need a local
				// rotation rather than being modified themselves, they should be filtered for separately.

				// Scale needs to affect the correct axes, but not invert the forward axis.
				let new_z = xform.scale.y;
				xform.scale.y = xform.scale.z;
				xform.scale.z = new_z;
			}
		});
	}
}

#[derive(Event)]
pub struct FinishedProcessing<A: Asset> {
	pub id: AssetId<A>,
}

pub fn make_new_gltf_scenes_z_up(
	mut load_events: EventReader<AssetEvent<Gltf>>,
	gltfs: Res<Assets<Gltf>>,
	mut assets: GltfSubAssets,
	mut finish_events: EventWriter<FinishedProcessing<Gltf>>,
) {
	for ev in load_events.read() {
		let &AssetEvent::LoadedWithDependencies { id } = ev else {
			continue;
		};
		info!(?id, "Making scene z-up");
		let gltf = gltfs.get(id).unwrap();
		let mut processor = gltf.process(&mut assets);
		processor.make_z_up();
		finish_events.send(FinishedProcessing { id });
	}
}

/// Returns a system that calls [Commands::set_state]. This lets the condition for moving to the
/// next state be clear at the point of configuring a schedule, rather than within the body of some
/// system.
///
/// ```
/// # use bevy::prelude::App;
/// # let mut app = App::new();
/// # #[derive(Resource)]
/// # struct MyAssets;
/// # app.add_resource(MyAssets);
/// # #[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
/// # enum MyStates { Loading, InGame }
/// app.add_systems(
///     Update,
///     set_state_to(MyStates::InGame)
///         .run_if(in_state(MyStates::Loading).and(resource_exists::<MyAssets>)),
/// );
/// ```
pub fn set_state_to<S: FreelyMutableState>(state: S) -> impl System<In = (), Out = ()> {
	IntoSystem::into_system(move |mut cmds: Commands| {
		cmds.set_state(state.clone());
	})
}

/// A run condition that returns `true` if at least one entity matches filter `F`.
pub fn entity_exists<F: QueryFilter>(q: Query<(), F>) -> bool {
	!q.is_empty()
}

/// Like [entity_exists], but may warn or panic (depending on `ParamWarnPolicy`) if more than one
/// entity matches filter `F`.
pub fn single_entity_exists<F: QueryFilter>(q: Option<Single<(), F>>) -> bool {
	q.is_some()
}

pub trait IntoSetupStep<M, F> {
	fn setup_done_when<CM>(self, done_condition: impl Condition<CM>) -> ScheduleConfigs<ScheduleSystem>;
}

impl<M, F> IntoSetupStep<M, F> for F
where
	F: SystemParamFunction<M, In = (), Out = ()> + Sized,
	M: 'static,
{
	fn setup_done_when<CM>(self, done_condition: impl Condition<CM>) -> ScheduleConfigs<ScheduleSystem> {
		// FIXME: never param warn, was moved to global in 0.16
		self.run_if(not(done_condition))
	}
}

mod asset_mut {
	use bevy::asset::{Asset, AssetId, Assets};
	use bevy::prelude::{DetectChanges, Mut, ResMut};

	pub struct AssetMut<'w, A: Asset> {
		assets: Mut<'w, Assets<A>>,
		id: AssetId<A>,
	}

	impl<'w, A: Asset> AssetMut<'w, A> {
		pub fn reborrow(&mut self) -> AssetMut<'_, A> {
			let assets = self.assets.reborrow();
			AssetMut {
				assets,
				id: self.id,
			}
		}

		pub fn remove(mut self) -> A {
			unsafe {
				// SAFETY: It is only possible to acquire an `AssetMut` if the asset exists.
				self.assets.remove(self.id).unwrap_unchecked()
			}
		}

		#[cfg(feature = "track_changes")]
		pub fn changed_by(&self) -> &std::panic::Location {
			self.assets.changed_by()
		}
	}

	impl<A: Asset> std::ops::Deref for AssetMut<'_, A> {
		type Target = A;
		fn deref(&self) -> &Self::Target {
			unsafe {
				// SAFETY: It is only possible to acquire an `AssetMut` if the asset exists.
				self.assets.get(self.id).unwrap_unchecked()
			}
		}
	}

	impl<A: Asset> std::ops::DerefMut for AssetMut<'_, A> {
		fn deref_mut(&mut self) -> &mut Self::Target {
			unsafe {
				// SAFETY: It is only possible to acquire an `AssetMut` if the asset exists.
				self.assets.get_mut(self.id).unwrap_unchecked()
			}
		}
	}

	pub trait BorrowAssetMut<A: Asset> {
		fn asset_mut(&mut self, id: AssetId<A>) -> Option<AssetMut<A>>;
	}

	impl<A: Asset> BorrowAssetMut<A> for ResMut<'_, Assets<A>> {
		fn asset_mut(&mut self, id: AssetId<A>) -> Option<AssetMut<A>> {
			if self.contains(id) {
				Some(AssetMut {
					assets: self.reborrow(),
					id,
				})
			} else {
				None
			}
		}
	}
}

pub use asset_mut::*;

#[cfg(feature = "track_changes")]
fn debug_changed_res<R: Resource>(res: Res<R>) {
	if res.is_changed() {
		debug!(
			"{} changed by {}",
			std::any::type_name::<R>(),
			res.changed_by()
		)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Aabb3dMeshBuilder {
	aabb: Aabb3d,
}

impl Aabb3dMeshBuilder {
	pub fn new(center: impl Into<Vec3A>, size: impl Into<Vec3A>) -> Self {
		Self {
			aabb: Aabb3d::new(center, size),
		}
	}
	pub fn from_corners(min: impl Into<Vec3A>, max: impl Into<Vec3A>) -> Self {
		Self {
			aabb: Aabb3d {
				min: min.into(),
				max: max.into(),
			},
		}
	}
}

impl From<Aabb3d> for Aabb3dMeshBuilder {
	fn from(aabb: Aabb3d) -> Self {
		Self { aabb }
	}
}

impl MeshBuilder for Aabb3dMeshBuilder {
	fn build(&self) -> Mesh {
		// Copied from CuboidMeshBuilder, just changed min and max

		let min = self.aabb.min;
		let max = self.aabb.max;

		// Suppose Y-up right hand, and camera look from +Z to -Z
		let vertices = &[
			// Front
			([min.x, min.y, max.z], [0.0, 0.0, 1.0], [0.0, 0.0]),
			([max.x, min.y, max.z], [0.0, 0.0, 1.0], [1.0, 0.0]),
			([max.x, max.y, max.z], [0.0, 0.0, 1.0], [1.0, 1.0]),
			([min.x, max.y, max.z], [0.0, 0.0, 1.0], [0.0, 1.0]),
			// Back
			([min.x, max.y, min.z], [0.0, 0.0, -1.0], [1.0, 0.0]),
			([max.x, max.y, min.z], [0.0, 0.0, -1.0], [0.0, 0.0]),
			([max.x, min.y, min.z], [0.0, 0.0, -1.0], [0.0, 1.0]),
			([min.x, min.y, min.z], [0.0, 0.0, -1.0], [1.0, 1.0]),
			// Right
			([max.x, min.y, min.z], [1.0, 0.0, 0.0], [0.0, 0.0]),
			([max.x, max.y, min.z], [1.0, 0.0, 0.0], [1.0, 0.0]),
			([max.x, max.y, max.z], [1.0, 0.0, 0.0], [1.0, 1.0]),
			([max.x, min.y, max.z], [1.0, 0.0, 0.0], [0.0, 1.0]),
			// Left
			([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
			([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
			([min.x, max.y, min.z], [-1.0, 0.0, 0.0], [0.0, 1.0]),
			([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
			// Top
			([max.x, max.y, min.z], [0.0, 1.0, 0.0], [1.0, 0.0]),
			([min.x, max.y, min.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
			([min.x, max.y, max.z], [0.0, 1.0, 0.0], [0.0, 1.0]),
			([max.x, max.y, max.z], [0.0, 1.0, 0.0], [1.0, 1.0]),
			// Bottom
			([max.x, min.y, max.z], [0.0, -1.0, 0.0], [0.0, 0.0]),
			([min.x, min.y, max.z], [0.0, -1.0, 0.0], [1.0, 0.0]),
			([min.x, min.y, min.z], [0.0, -1.0, 0.0], [1.0, 1.0]),
			([max.x, min.y, min.z], [0.0, -1.0, 0.0], [0.0, 1.0]),
		];

		let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
		let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
		let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

		let indices = Indices::U32(vec![
			0, 1, 2, 2, 3, 0, // front
			4, 5, 6, 6, 7, 4, // back
			8, 9, 10, 10, 11, 8, // right
			12, 13, 14, 14, 15, 12, // left
			16, 17, 18, 18, 19, 16, // top
			20, 21, 22, 22, 23, 20, // bottom
		]);

		Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::default(),
		)
		.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
		.with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
		.with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
		.with_inserted_indices(indices)
	}
}
