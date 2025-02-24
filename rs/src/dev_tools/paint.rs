use crate::ui::egui;
use crate::{
	game::mtn::{
		BvhContents, BvhNode, MeshGraph, MountainAssets, MountainPeak, MountainScene, Ray3dExt,
		TriangleIndex, debug_find_triangle_intersected_by_ray, find_triangle_intersected_by_ray,
	},
	game::{GameLoadingState, GameSetupKey, GameSetupLabel},
	game::{
		mtn::terrain::{
			ATTRIBUTE_TERRAIN_WEIGHTS_0_3, ATTRIBUTE_TERRAIN_WEIGHTS_4_7, TerrainKind,
			TerrainTexturesStacked,
		},
		mtn::{Mountain, MountainHydrated, MountainSceneSpawned},
		tools::{ActiveTool, ReflectTool, Tool, active_tool_is},
	},
	setup_tracking::{self, IntoDependencyProvider, Progress, RegisterProvider},
	state::GlobalState,
	ui::egui::{Id, Ui},
	util::{BorrowAssetMut, MeshExt, log_errors},
};
use bevy::input::keyboard::KeyboardInput;
use bevy::reflect::TypeRegistry;
use bevy::render::mesh::MeshVertexAttribute;
use bevy::{
	color::palettes::basic::YELLOW,
	ecs::system::SystemId,
	input::mouse::AccumulatedMouseMotion,
	math::bounding::BoundingVolume,
	picking::mesh_picking::ray_cast::{Backfaces, ray_mesh_intersection},
	prelude::*,
	reflect::{FromType, GetTypeRegistration, TypeData},
	render::mesh::allocator::MeshAllocatorSettings,
	render::mesh::{Indices, VertexAttributeValues},
	render::render_resource::{Face, VertexFormat},
	tasks::{AsyncComputeTaskPool, futures_lite},
	utils::HashMap,
	window::PrimaryWindow,
};
use bevy_egui::EguiContexts;
use bevy_inspector_egui::{
	egui::emath::One,
	inspector_egui_impls::InspectorPrimitive,
	inspector_options::std_options::{NumberDisplay, NumberOptions},
	inspector_options::{InspectorOptionsType, Target},
	prelude::*,
	quick::ResourceInspectorPlugin,
	reflect_inspector::InspectorUi,
};
use enum_map::{EnumMap, enum_map};
use gltf::Glb;
use gltf::binary::Header;
use gltf_json::validation::{self, Checked::Valid, USize64};
use serde::Serialize;
use smol::{fs::File, fs::OpenOptions, io::AsyncWriteExt, io::BufWriter};
use std::time::Instant;
use std::{
	any::Any, borrow::Cow, cmp::Ordering, cmp::max, collections::BTreeMap, error::Error,
	fmt::Debug, ops::RangeInclusive, path::Path, path::PathBuf, time::Duration,
};
use strum::{EnumCount, VariantArray};
use tiny_bail::prelude::r;

pub struct TerrainPaintPlugin;

crate::new_game_setup_label!(
	PaintingReady,
	setup_tracking::resource_progress::<TerrainWeights>
);

impl Plugin for TerrainPaintPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Paintbrush>()
			.register_type::<CurveStart<f32>>()
			.register_type::<CurveEnd<f32>>()
			.register_type::<CurveStart<Vec3>>()
			.register_type::<CurveEnd<Vec3>>()
			.register_type::<BrushShape<f32>>()
			.register_type::<BrushShape<Vec3>>()
			.register_type::<TerrainWeightBrush>()
			.register_type::<SculptSpace>()
			.register_type::<BrushEaseFn>()
			.insert_resource(MeshAllocatorSettings {
				max_slab_size: 4096 * 1024 * 1024,
				large_threshold: 1024 * 1024 * 1024,
				..default()
			})
			.register_provider(
				setup_painting
					.provides([PaintingReady.intern()])
					.requires([MountainHydrated.intern()]),
			)
			.add_systems(
				First,
				TerrainWeights::update_on_mesh_reloaded.never_param_warn(),
			)
			.add_systems(
				Update,
				(
					paint_terrain,
					redistribute_weights.run_if(resource_exists_and_changed::<TerrainWeights>),
					apply_weights_to_mesh,
					Paintbrush::draw_ui,
					Paintbrush::open_close,
				)
					.run_if(in_state(GlobalState::InGame)),
			)
			.add_systems(Last, save_on_exit.never_param_warn())
			.add_systems(OnExit(GlobalState::InGame), |mut cmds: Commands| {
				cmds.remove_resource::<TerrainWeights>()
			});
	}
}

pub fn setup_painting(mut cmds: Commands) {
	cmds.init_resource::<TerrainWeights>();
}

pub fn paint_terrain(
	btns: Res<ButtonInput<MouseButton>>,
	window: Query<&Window, With<PrimaryWindow>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mtn: Single<(&Mesh3d, &GlobalTransform, &MeshGraph), With<Mountain>>,
	peak: Single<&Transform, With<MountainPeak>>,
	cam: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
	tool: Res<ActiveTool>,
	mut weight_map: ResMut<TerrainWeights>,
	mut gizmos: Gizmos,
	keys: Res<ButtonInput<KeyCode>>,
	t: Res<Time>,
	mut last_brush_global_pos: Local<Isometry3d>,
	mut painted_this_stroke: Local<HashMap<usize, u16>>,
	mut debug_bvh: Local<bool>,
	mut debug_raycast: Local<bool>,
	mut save_timer: Local<Option<Timer>>,
) {
	let dt = t.delta();
	let save_timer = save_timer.get_or_insert_with(|| {
		let dur = Duration::from_secs(2);
		let mut timer = Timer::new(dur, TimerMode::Once);
		timer.set_elapsed(dur);
		timer.tick(dt); // Sets `finished`
		timer.tick(dt); // Prevents `just_finished` from triggering immediately
		timer
	});

	let save = |mesh: Mesh, peak: Transform| {
		AsyncComputeTaskPool::get()
			.spawn(mtn_gltf_save_task(
				"../assets/terrain/mtn.glb".into(),
				mesh,
				peak,
			))
			.detach();
	};

	let Some(brush) = tool.downcast_ref::<Paintbrush>() else {
		if !save_timer.finished() {
			let mesh_handle = mtn.0;
			let mut mesh = r!(meshes.asset_mut(mesh_handle.id()));
			let dur = save_timer.duration();
			save_timer.set_elapsed(dur);
			save_timer.tick(dt); // Sets `finished`
			save_timer.tick(dt); // Clears `just_finished`
			save(mesh.clone(), **peak);
		}
		painted_this_stroke.clear();
		return;
	};

	let window = r!(window.get_single());

	if keys.just_pressed(KeyCode::F7) {
		*debug_bvh = !*debug_bvh;
	}
	if keys.just_pressed(KeyCode::F8) {
		*debug_raycast = !*debug_raycast;
	}

	let (mesh_handle, xform, graph) = *mtn;
	let mut mesh = r!(meshes.asset_mut(mesh_handle.id()));

	if *debug_bvh {
		bvh_gizmos(&graph.bvh.root, &mut gizmos, 0);
	}

	save_timer.tick(t.delta());
	if btns.just_released(MouseButton::Left) {
		mesh.compute_normals();
		painted_this_stroke.clear();
		save_timer.reset();
	}

	let Some(cursor_pos) = window.cursor_position() else {
		return;
	};
	let (cam, cam_xform) = *cam;
	let Ok(ray) = cam.viewport_to_world(cam_xform, cursor_pos) else {
		return;
	};
	if let Some((tri_idx, t)) = if *debug_raycast {
		let mut tris = Vec::new();
		let hit = debug_find_triangle_intersected_by_ray(
			ray,
			&*mesh,
			graph,
			Some(Face::Back),
			|node, depth| {
				let size = node.aabb.half_size() * 2.0;
				gizmos.cuboid(
					Transform {
						translation: node.aabb.center().into(),
						scale: Vec3::new(size.x, size.y, size.z),
						..default()
					},
					Color::hsv(((depth * 32) % 360) as f32, 1.0, 1.0),
				)
			},
			|tri_set| {
				tris.extend(tri_set.iter().copied());
			},
		);
		for idx in tris {
			let tri = idx.triangle(&*mesh);
			let (color, offset) = if hit.unzip().0 == Some(idx) {
				(
					bevy::color::palettes::basic::LIME.into(),
					tri.normal().unwrap() * 0.01,
				)
			} else {
				(Color::WHITE, Vec3::ZERO)
			};
			let [a, b, c] = tri.vertices;
			for side in [(a, b), (b, c), (c, a)] {
				gizmos.line(side.0 + offset, side.1 + offset, color);
			}
		}
		hit
	} else {
		ray.find_intersected_triangle(&*mesh, graph, Some(Face::Back))
	} {
		let tri = tri_idx.triangle(&*mesh);
		let norm = tri.normal().unwrap();
		let point = ray.origin + (ray.direction * t);
		let brush_global_pos = Isometry3d {
			rotation: Quat::from_rotation_arc(Vec3::NEG_Z, *norm),
			translation: point.into(),
		};
		let gizmo_iso = Isometry3d {
			rotation: brush_global_pos.rotation,
			translation: (point + (norm * 0.1)).into(),
		};
		let radius = match &*brush {
			Paintbrush::TerrainKindWeights { radius, .. } | Paintbrush::Sculpt { radius, .. } => {
				*radius
			}
		};
		gizmos.circle(gizmo_iso, radius, YELLOW);

		if btns.pressed(MouseButton::Left) {
			save_timer.reset();
			brush.paint(
				&mut *mesh,
				brush_global_pos,
				*last_brush_global_pos,
				ray,
				tri_idx,
				norm,
				*xform,
				&mut *painted_this_stroke,
				weight_map.reborrow(),
			)
		}
		*last_brush_global_pos = brush_global_pos;
	} else {
		match &*brush {
			Paintbrush::TerrainKindWeights { radius, .. } | Paintbrush::Sculpt { radius, .. } => {
				let gizmo_iso =
					*last_brush_global_pos * Isometry3d::from_translation(Vec3::Z * 0.1);
				gizmos.circle(gizmo_iso, *radius, YELLOW)
			}
		};
	}

	if save_timer.just_finished() {
		save(mesh.clone(), **peak)
	}
}

fn bvh_gizmos(node: &BvhNode<TriangleIndex>, gizmos: &mut Gizmos, depth: usize) {
	use bevy::math::bounding::BoundingVolume;
	let size = node.aabb.half_size() * 2.0;
	gizmos.cuboid(
		Transform {
			translation: node.aabb.center().into(),
			scale: Vec3::new(size.x, size.y, size.z),
			..default()
		},
		Color::hsv(((depth * 32) % 360) as f32, 1.0, 1.0),
	);
	match &node.contents {
		BvhContents::Branch(subtrees) => {
			for subtree in subtrees.iter() {
				bvh_gizmos(subtree, gizmos, depth + 1);
			}
		}
		_ => {}
	}
}

#[derive(Debug, Reflect, InspectorOptions)]
#[reflect(Tool, Default, InspectorOptions)]
pub enum Paintbrush {
	TerrainKindWeights {
		#[reflect(default = "default_radius")]
		#[inspector(min = f32::EPSILON)]
		radius: f32,
		weights: TerrainWeightBrush,
		erase: bool,
	},
	Sculpt {
		#[reflect(default = "default_radius")]
		radius: f32,
		shape: BrushShape<Vec3>,
		basis: SculptSpace,
	},
}

impl Tool for Paintbrush {}

fn default_radius() -> f32 {
	3.0
}

impl Default for Paintbrush {
	fn default() -> Self {
		Self::TerrainKindWeights {
			radius: 3.0,
			weights: TerrainWeightBrush {
				kind: TerrainKind::Grass,
				shape: default(),
			},
			erase: false,
		}
	}
}

impl Paintbrush {
	pub fn paint(
		&self,
		mut mesh: &mut Mesh,
		brush_global_pos: Isometry3d,
		prev_brush_global_pos: Isometry3d,
		ray: Ray3d,
		hit_tri: TriangleIndex,
		hit_normal: Dir3,
		mesh_xform: GlobalTransform,
		painted_this_stroke: &mut HashMap<usize, u16>,
		mut weight_map: Mut<TerrainWeights>,
	) {
		let brush_global_pos = Vec3::from(brush_global_pos.translation);
		let prev_brush_global_pos = Vec3::from(prev_brush_global_pos.translation);
		let stroke_vector = brush_global_pos - prev_brush_global_pos;
		let stroke_dir = stroke_vector.normalize_or_zero();

		let dist_to_stroke = |vert_global_pos: Vec3| {
			let dot = (vert_global_pos - prev_brush_global_pos).dot(stroke_dir);
			let projection = if dot > stroke_vector.length() {
				brush_global_pos
			} else if dot < 0.0 {
				prev_brush_global_pos
			} else {
				(dot * stroke_dir) + prev_brush_global_pos
			};
			vert_global_pos.distance(projection)
		};

		match self {
			&Paintbrush::TerrainKindWeights {
				radius,
				ref weights,
				erase,
			} => {
				let positions = mesh.positions().unwrap();
				for (i, dist) in positions
					.iter()
					.copied()
					.map(Vec3::from_array)
					.enumerate()
					.filter_map(|(i, pos)| {
						let vert_global_pos = mesh_xform * pos;
						let dist = dist_to_stroke(vert_global_pos);
						(dist <= radius).then_some((i, dist))
					}) {
					let TerrainWeightBrush { kind, shape } = weights;
					let w = dist / radius;
					let falloff: f32 = shape.sample(w).unwrap();
					let mut w = (falloff * u16::MAX as f32) as u16;
					painted_this_stroke
						.entry(i)
						.and_modify(|prev| {
							if w > *prev {
								let diff = w - *prev;
								*prev = w;
								w = diff;
							} else {
								w = 0;
							}
						})
						.or_insert(w);
					if w > 0 {
						let out = &mut weight_map.map[*kind][i];
						if erase {
							*out = out.saturating_sub(w);
						} else {
							*out = out.saturating_add(w);
						}
					}
				}
			}
			Paintbrush::Sculpt {
				radius,
				shape: falloff,
				basis,
			} => {
				let n = mesh.positions().unwrap().len();
				for i in 0..n {
					let normal = Vec3::from_array(mesh.normals().unwrap()[i]);
					let position = &mut mesh.positions_mut().unwrap()[i];
					let vert_global_pos = mesh_xform * Vec3::from_array(*position);
					let dist = dist_to_stroke(vert_global_pos);
					if dist > *radius {
						continue;
					}
					let offset = falloff.sample(dist / radius).unwrap();
					// TODO: Should probably be per-axis, but would need to change type of `painted_this_stroke`
					let mut w = (u16::MAX as f32 * offset.length()
						/ falloff.range().start().length()) as u16;
					painted_this_stroke
						.entry(i)
						.and_modify(|prev| {
							if w > *prev {
								let diff = w - *prev;
								*prev = w;
								w = diff;
							} else {
								w = 0;
							}
						})
						.or_insert(w);
					let w = w as f32 / (u16::MAX as f32 * falloff.range().start().length());
					if w > f32::EPSILON {
						let offset = offset * w;
						let offset = match basis {
							SculptSpace::Tangent => {
								Quat::from_rotation_arc(Vec3::Z, normal) * offset
							}
							SculptSpace::Model => offset,
							SculptSpace::Camera => {
								let z = mesh_xform.rotation().inverse() * -ray.direction;
								Quat::from_rotation_arc(Vec3::Z, *z) * offset
							}
						};
						*position = (Vec3::from_array(*position) + offset).to_array()
					}
				}
			}
		}
	}

	pub fn draw_ui(
		mut contexts: EguiContexts,
		mut tool: ResMut<ActiveTool>,
		reg: Res<AppTypeRegistry>,
	) {
		let brush = tool.downcast_mut::<Paintbrush>();
		let ctx = contexts.ctx_mut();
		let was_open = brush.is_some();
		let mut open = was_open;
		egui::Window::new("Paintbrush")
			.open(&mut open)
			.show(ctx, |ui| {
				if let Some(brush) = brush {
					bevy_inspector_egui::reflect_inspector::ui_for_value(brush, ui, &*reg.read());
				}
			});
		if was_open && !open {
			r!(tool.try_take_as::<Paintbrush>());
		}
	}

	pub fn open_close(
		keys: Res<ButtonInput<KeyCode>>,
		mouse: Res<ButtonInput<MouseButton>>,
		mut tool: ResMut<ActiveTool>,
	) {
		let open = tool.is::<Paintbrush>();
		if !open && keys.just_pressed(KeyCode::KeyP) {
			tool.open(Paintbrush::default())
		} else if open && mouse.just_pressed(MouseButton::Right) {
			r!(tool.try_take_as::<Paintbrush>());
		}
	}
}

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq, Eq)]
#[reflect(Default)]
pub enum SculptSpace {
	/// Move vertices in tangent space (treating their normals as +Z).
	#[default]
	Tangent,
	/// Move vertices in local model space.
	Model,
	/// Move vertices treating +Z as pointing at the camera.
	Camera,
}

#[derive(Debug, Clone, Reflect)]
#[reflect(Default, InspectorOptions, where T: ZeroAndOne + Clone + Send + Sync + 'static, InspectorOptions: FromType<Self>)]
pub enum BrushShape<T> {
	Ease {
		start: CurveStart<T>,
		end: CurveEnd<T>,
		ease_fn: BrushEaseFn,
	},
	Constant(CurveStart<T>),
}

impl<T: ZeroAndOne + Clone> BrushShape<T> {
	pub fn range(&self) -> RangeInclusive<T> {
		match self {
			BrushShape::Ease { start, end, .. } => start.0.clone()..=end.0.clone(),
			BrushShape::Constant(val) => val.0.clone()..=val.0.clone(),
		}
	}
}

#[derive(Debug, Clone, Reflect, Deref, DerefMut)]
#[reflect(Default, where T: ZeroAndOne)]
pub struct CurveStart<T>(T);

impl<T: ZeroAndOne> Default for CurveStart<T> {
	fn default() -> Self {
		Self(T::one())
	}
}

impl<T: ZeroAndOne> ZeroAndOne for CurveStart<T> {
	fn zero() -> Self {
		Self(T::zero())
	}
	fn one() -> Self {
		Self(T::one())
	}
}

#[derive(Debug, Clone, Reflect, Deref, DerefMut)]
#[reflect(Default, where T: ZeroAndOne)]
pub struct CurveEnd<T>(T);

impl<T: ZeroAndOne> Default for CurveEnd<T> {
	fn default() -> Self {
		Self(T::zero())
	}
}

impl<T: ZeroAndOne> ZeroAndOne for CurveEnd<T> {
	fn zero() -> Self {
		Self(T::zero())
	}
	fn one() -> Self {
		Self(T::one())
	}
}

impl<T: ZeroAndOne + Clone + Send + Sync + 'static> Default for BrushShape<T> {
	fn default() -> Self {
		Self::Ease {
			start: CurveStart::one(),
			end: CurveEnd::zero(),
			ease_fn: BrushEaseFn(EaseFunction::CircularIn),
		}
	}
}

trait ZeroAndOne {
	fn zero() -> Self;
	fn one() -> Self;
}

impl ZeroAndOne for f32 {
	fn zero() -> Self {
		0.0
	}

	fn one() -> Self {
		1.0
	}
}

impl ZeroAndOne for Vec3 {
	fn zero() -> Self {
		Self::ZERO
	}

	fn one() -> Self {
		Self::Z
	}
}

///
#[derive(Debug, Reflect, Clone, Copy, Deref, DerefMut)]
#[reflect(Default)]
pub struct BrushEaseFn(pub EaseFunction);

impl Default for BrushEaseFn {
	fn default() -> Self {
		BrushEaseFn(EaseFunction::CircularIn)
	}
}

impl FromType<BrushShape<f32>> for InspectorOptions {
	fn from_type() -> Self {
		let mut options = InspectorOptions::default();

		let mut inner_opts = NumberOptions::<f32>::between(0.0, 1.0).with_speed(0.002);
		inner_opts.display = NumberDisplay::Slider;
		let mut field_opts = InspectorOptions::default();
		field_opts.insert(Target::Field(0), inner_opts);

		// Ease
		// Ease::start
		options.insert(
			Target::VariantField {
				variant_index: 0,
				field_index: 0,
			},
			field_opts.clone(),
		);
		// Ease::end
		options.insert(
			Target::VariantField {
				variant_index: 0,
				field_index: 1,
			},
			field_opts.clone(),
		);

		// Constant
		options.insert(
			Target::VariantField {
				variant_index: 1,
				field_index: 0,
			},
			field_opts,
		);

		options
	}
}

impl FromType<BrushShape<Vec3>> for InspectorOptions {
	fn from_type() -> Self {
		let mut options = InspectorOptions::default();

		// let mut inner_opts = NumberOptions::<f32>::between(
		// 	0.0,
		// 	1.0,
		// ).with_speed(0.002);
		// inner_opts.display = NumberDisplay::Slider;
		// let mut field_opts = InspectorOptions::default();
		// field_opts.insert(
		// 	Target::Field(0),
		// 	inner_opts,
		// );
		//
		// // Ease
		// // Ease::start
		// options.insert(
		// 	Target::VariantField {
		// 		variant_index: 0,
		// 		field_index: 0,
		// 	},
		// 	field_opts.clone(),
		// );
		// // Ease::end
		// options.insert(
		// 	Target::VariantField {
		// 		variant_index: 0,
		// 		field_index: 1,
		// 	},
		// 	field_opts.clone(),
		// );
		//
		// // Constant
		// options.insert(
		// 	Target::VariantField {
		// 		variant_index: 1,
		// 		field_index: 0,
		// 	},
		// 	field_opts,
		// );

		options
	}
}

impl<T: Clone> Curve<T> for BrushShape<T>
where
	EasingCurve<T>: Curve<T>,
{
	fn domain(&self) -> Interval {
		match self {
			BrushShape::Ease {
				start,
				end,
				ease_fn,
			} => EasingCurve::new(start.0.clone(), end.0.clone(), ease_fn.0.clone()).domain(),
			BrushShape::Constant(_) => Interval::EVERYWHERE,
		}
	}

	fn sample_unchecked(&self, t: f32) -> T {
		match self {
			BrushShape::Ease {
				start,
				end,
				ease_fn,
			} => EasingCurve::new(start.0.clone(), end.0.clone(), ease_fn.0.clone())
				.sample_unchecked(t),
			BrushShape::Constant(val) => val.0.clone(),
		}
	}
}

#[derive(Debug, Default, Reflect, Clone, InspectorOptions)]
#[reflect(Default, InspectorOptions)]
pub struct TerrainWeightBrush {
	pub kind: TerrainKind,
	pub shape: BrushShape<f32>,
}

#[derive(Resource, Debug)]
pub struct TerrainWeights {
	num_vertices: usize,
	map: EnumMap<TerrainKind, Box<[u16]>>,
}

impl FromWorld for TerrainWeights {
	fn from_world(world: &mut World) -> Self {
		let mut q = world.query_filtered::<&Mesh3d, With<Mountain>>();
		let mtn_mesh = q.single(world).0.id();
		let mtn_mesh = world.resource::<Assets<Mesh>>().get(mtn_mesh).unwrap();

		Self::from_mesh(mtn_mesh)
	}
}

impl TerrainWeights {
	fn from_mesh(mesh: &Mesh) -> Self {
		let Some(VertexAttributeValues::Unorm16x4(weights_0_3)) =
			mesh.attribute(ATTRIBUTE_TERRAIN_WEIGHTS_0_3)
		else {
			unreachable!();
		};
		let Some(VertexAttributeValues::Unorm16x4(weights_4_7)) =
			mesh.attribute(ATTRIBUTE_TERRAIN_WEIGHTS_4_7)
		else {
			unreachable!();
		};
		let num_vertices = weights_0_3.len();

		let map = EnumMap::from_fn(|kind| {
			let k = kind as usize;
			(0..num_vertices)
				.map(|i| {
					if k < 4 {
						weights_0_3[i][k]
					} else if k < 8 {
						weights_4_7[i][k - 4]
					} else {
						unreachable!();
					}
				})
				.collect()
		});

		Self { num_vertices, map }
	}

	fn update_on_mesh_reloaded(
		mut cmds: Commands,
		mtn_mesh: Single<&Mesh3d, With<Mountain>>,
		meshes: Res<Assets<Mesh>>,
		mut events: EventReader<AssetEvent<Mesh>>,
	) {
		for ev in events.read() {
			match ev {
				AssetEvent::LoadedWithDependencies { id } => {
					if *id == mtn_mesh.0.id() {
						info!("Updating TerrainWeights because mesh was reloaded");
						cmds.insert_resource(Self::from_mesh(meshes.get(*id).unwrap()));
					}
				}
				_ => {}
			}
		}
	}
}

pub fn redistribute_weights(mut weight_map: ResMut<TerrainWeights>, tool: Res<ActiveTool>) {
	let brush = tool.downcast_ref::<Paintbrush>();
	for i in 0..weight_map.num_vertices {
		let mut weights = weight_map
			.map
			.iter()
			.map(|(k, v)| (k, v[i]))
			.collect::<Vec<_>>();
		let mut sum: u64 = weights.iter().map(|(_, w)| *w as u64).sum();
		if sum == 0 {
			weight_map.map[TerrainKind::Dirt][i] = u16::MAX;
			for k in &TerrainKind::VARIANTS[1..] {
				weight_map.map[*k][i] = 0;
			}
			continue
		}
		if sum < u16::MAX as u64 {
			let factor = u16::MAX as f64 / sum as f64;
			for (_, w) in weights.iter_mut() {
				*w = (*w as f64 * factor).ceil() as u16;
			}
			sum = weights.iter().map(|(_, w)| *w as u64).sum();
			debug_assert!(sum >= u16::MAX as u64, ".ceil() above should prevent this");
		}
		while sum > u16::MAX as u64 {
			let rem = sum - u16::MAX as u64;
			if let Some(Paintbrush::TerrainKindWeights {
				weights: brush_weights,
				..
			}) = &brush
			{
				weights.sort_by(|(ka, wa), (kb, wb)| {
					if brush_weights.kind == *kb {
						// Don't keep overriding brush strokes
						Ordering::Less
					} else if brush_weights.kind == *ka {
						Ordering::Greater
					} else {
						wa.cmp(wb)
					}
				});
			} else {
				weights.sort_by_key(|(kind, weight)| *weight)
			}
			if rem > weights[0].1 as u64 {
				sum -= weights[0].1 as u64;
				weights.remove(0);
				if weight_map.map[weights[0].0][i] != 0 {
					weight_map.map[weights[0].0][i] = 0;
				}
			} else {
				weights[0].1 -= rem as u16;
				sum -= rem;
			}
		}
		for (k, weight) in weights {
			if weight_map.map[k][i] != weight {
				weight_map.map[k][i] = weight;
			}
		}
	}
}

pub fn apply_weights_to_mesh(
	weight_map: Res<TerrainWeights>,
	mtn: Single<&Mesh3d, With<Mountain>>,
	mut meshes: ResMut<Assets<Mesh>>,
	t: Res<Time>,
	mut update_timer: Local<Timer>,
) {
	// Don't update mesh too quickly or framerate drops massively
	if weight_map.is_changed() && update_timer.finished() {
		update_timer.set_duration(Duration::from_millis(33));
		update_timer.reset();
	} else {
		update_timer.tick(t.delta());
	}
	if update_timer.just_finished() {
		debug!("Applying weights to mesh");
		let mesh = meshes.get_mut(&mtn.0).unwrap();
		for i in 0..weight_map.num_vertices {
			let Some(VertexAttributeValues::Unorm16x4(weights)) =
				mesh.attribute_mut(ATTRIBUTE_TERRAIN_WEIGHTS_0_3)
			else {
				unreachable!()
			};
			for j in 0..4 {
				weights[i][j] = weight_map.map[TerrainKind::VARIANTS[j]][i];
			}
			let Some(VertexAttributeValues::Unorm16x4(weights)) =
				mesh.attribute_mut(ATTRIBUTE_TERRAIN_WEIGHTS_4_7)
			else {
				unreachable!()
			};
			for j in 4..TerrainKind::COUNT as usize {
				weights[i][j - 4] = weight_map.map[TerrainKind::VARIANTS[j]][i];
			}
		}
	}
}

pub fn save_mtn_gltf(
	path: In<PathBuf>,
	mtn: Single<&Mesh3d, With<Mountain>>,
	peak: Single<&Transform, With<MountainPeak>>,
	meshes: Res<Assets<Mesh>>,
) {
	let mesh = r!(meshes.get(&mtn.0)).clone();
	let peak_xform = peak.clone();
	AsyncComputeTaskPool::get()
		.spawn(async move { r!(mtn_gltf_save_task(path.0, mesh, peak_xform).await) })
		.detach();
}

pub fn save_on_exit(
	mut cmds: Commands,
	mtn: Single<&Mesh3d, With<Mountain>>,
	peak: Single<&Transform, With<MountainPeak>>,
	meshes: Res<Assets<Mesh>>,
	mut exit_events: EventReader<AppExit>,
) {
	if let Some(_) = exit_events.read().next() {
		let mesh = r!(meshes.get(&mtn.0)).clone();
		let peak_xform = peak.clone();
		cmds.queue(move |_world: &mut World| {
			r!(smol::block_on(mtn_gltf_save_task(
				"../assets/terrain/mtn.glb".into(),
				mesh,
				peak_xform,
			)))
		});
	}
}

async fn mtn_gltf_save_task(
	path: PathBuf,
	mesh: Mesh,
	peak_xform: Transform,
) -> std::io::Result<()> {
	info!("Saving {path:?}");
	use gltf::binary::{Glb, Header};
	use gltf_json::validation::{Checked::Valid, USize64};

	// TODO: Deduplicate code by building a Gltf asset and re-using GlbSaver

	#[derive(Copy, Clone, Debug, bytemuck::NoUninit)]
	#[repr(C)]
	struct MountainVertex {
		position: [f32; 3],
		normal: [f32; 3],
		weights_0_3: [u16; 4],
		weights_4_7: [u16; 4],
	}

	fn into_other_io_err(e: impl Into<Box<dyn Error + Send + Sync>>) -> std::io::Error {
		std::io::Error::new(std::io::ErrorKind::Other, e)
	}

	fn attr_err(attr: &MeshVertexAttribute) -> std::io::Error {
		std::io::Error::new(
			std::io::ErrorKind::Other,
			format!("invalid/missing {}", attr.name),
		)
	}

	let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return Err(attr_err(&Mesh::ATTRIBUTE_POSITION));
	};
	let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
	else {
		return Err(attr_err(&Mesh::ATTRIBUTE_NORMAL));
	};
	let weights_0_3 = mesh.attribute(ATTRIBUTE_TERRAIN_WEIGHTS_0_3);
	let Some(VertexAttributeValues::Unorm16x4(weights_0_3)) = weights_0_3 else {
		return Err(attr_err(&ATTRIBUTE_TERRAIN_WEIGHTS_0_3));
	};
	let weights_4_7 = mesh.attribute(ATTRIBUTE_TERRAIN_WEIGHTS_4_7);
	let Some(VertexAttributeValues::Unorm16x4(weights_4_7)) = weights_4_7 else {
		return Err(attr_err(&ATTRIBUTE_TERRAIN_WEIGHTS_4_7));
	};
	let num_verts = positions.len();
	let mut min_pos = Vec3::MAX;
	let mut max_pos = Vec3::MIN;
	let vert_buffer = positions
		.iter()
		.zip(normals)
		.zip(weights_0_3.iter().zip(weights_4_7))
		.map(|((&position, &normal), (&weights_0_3, &weights_4_7))| {
			let position = [position[0], position[2], -position[1]];
			let normal = [normal[0], normal[2], -normal[1]];
			min_pos = Vec3::min(min_pos, Vec3::from_array(position));
			max_pos = Vec3::max(max_pos, Vec3::from_array(position));
			MountainVertex {
				position,
				normal,
				weights_0_3,
				weights_4_7,
			}
		})
		.collect::<Vec<_>>();
	let vert_bytes: &[u8] = bytemuck::cast_slice(&vert_buffer);
	let count = USize64::from(num_verts);

	let Some(indices) = mesh.indices() else {
		return Err(std::io::Error::new(
			std::io::ErrorKind::Other,
			"Mesh is missing indices",
		));
	};
	let indices_count = indices.len();
	let (indices_stride, indices_type, indices_bytes): (_, _, &[u8]) = match indices {
		Indices::U16(indices) => (
			gltf_json::buffer::Stride(size_of::<u16>()),
			gltf_json::accessor::GenericComponentType(gltf_json::accessor::ComponentType::U16),
			bytemuck::cast_slice(&indices),
		),
		Indices::U32(indices) => (
			gltf_json::buffer::Stride(size_of::<u32>()),
			gltf_json::accessor::GenericComponentType(gltf_json::accessor::ComponentType::U32),
			bytemuck::cast_slice(&indices),
		),
	};
	let mut whole_buffer = Vec::with_capacity(vert_bytes.len() + indices_bytes.len());
	whole_buffer.extend_from_slice(vert_bytes);
	debug_assert_eq!(whole_buffer.len() % 4, 0);
	debug_assert_eq!(whole_buffer.len(), num_verts * size_of::<MountainVertex>());
	whole_buffer.extend_from_slice(indices_bytes);
	let buf_len = whole_buffer.len();
	while whole_buffer.len() % 4 != 0 {
		whole_buffer.push(0);
	}

	let mut root = gltf_json::Root::default();

	let buffer = root.push(gltf_json::Buffer {
		byte_length: USize64::from(buf_len),
		name: None,
		uri: None,
		extensions: None,
		extras: None,
	});
	let buffer_view = root.push(gltf_json::buffer::View {
		buffer,
		byte_length: USize64::from(buf_len),
		byte_offset: None,
		byte_stride: Some(gltf_json::buffer::Stride(size_of::<MountainVertex>())),
		name: None,
		target: Some(Valid(gltf_json::buffer::Target::ArrayBuffer)),
		extensions: None,
		extras: None,
	});

	let positions = root.push(gltf_json::Accessor {
		buffer_view: Some(buffer_view),
		byte_offset: Some(USize64(0)),
		count,
		component_type: Valid(gltf_json::accessor::GenericComponentType(
			gltf_json::accessor::ComponentType::F32,
		)),
		extensions: None,
		extras: None,
		type_: Valid(gltf_json::accessor::Type::Vec3),
		min: Some(gltf_json::Value::from(Vec::from(min_pos.to_array()))),
		max: Some(gltf_json::Value::from(Vec::from(max_pos.to_array()))),
		name: None,
		normalized: false,
		sparse: None,
	});
	let normals = root.push(gltf_json::Accessor {
		buffer_view: Some(buffer_view),
		byte_offset: Some(USize64(std::mem::offset_of!(MountainVertex, normal) as u64)),
		count,
		component_type: Valid(gltf_json::accessor::GenericComponentType(
			gltf_json::accessor::ComponentType::F32,
		)),
		extensions: None,
		extras: None,
		type_: Valid(gltf_json::accessor::Type::Vec3),
		min: Some(gltf_json::Value::from(vec![-1.0; 3])),
		max: Some(gltf_json::Value::from(vec![1.0; 3])),
		name: None,
		normalized: false,
		sparse: None,
	});
	let weights_0_3 = root.push(gltf_json::Accessor {
		buffer_view: Some(buffer_view),
		byte_offset: Some(USize64(
			std::mem::offset_of!(MountainVertex, weights_0_3) as u64
		)),
		count,
		component_type: Valid(gltf_json::accessor::GenericComponentType(
			gltf_json::accessor::ComponentType::U16,
		)),
		extensions: None,
		extras: None,
		type_: Valid(gltf_json::accessor::Type::Vec4),
		min: None,
		max: None,
		name: None,
		normalized: true,
		sparse: None,
	});
	let weights_4_7 = root.push(gltf_json::Accessor {
		buffer_view: Some(buffer_view),
		byte_offset: Some(USize64(
			std::mem::offset_of!(MountainVertex, weights_4_7) as u64
		)),
		count,
		component_type: Valid(gltf_json::accessor::GenericComponentType(
			gltf_json::accessor::ComponentType::U16,
		)),
		extensions: None,
		extras: None,
		type_: Valid(gltf_json::accessor::Type::Vec4),
		min: None,
		max: None,
		name: None,
		normalized: true,
		sparse: None,
	});

	let indices_view = root.push(gltf_json::buffer::View {
		buffer,
		byte_length: USize64(indices_bytes.len() as u64),
		byte_offset: Some(USize64(vert_bytes.len() as u64)),
		byte_stride: Some(indices_stride),
		name: None,
		target: None,
		extensions: None,
		extras: None,
	});
	let indices = root.push(gltf_json::Accessor {
		buffer_view: Some(indices_view),
		byte_offset: Some(USize64(0)),
		count: USize64(indices_count as u64),
		component_type: Valid(indices_type),
		extensions: None,
		extras: None,
		type_: Valid(gltf_json::accessor::Type::Scalar),
		min: None,
		max: None,
		name: None,
		normalized: false,
		sparse: None,
	});

	let primitive = gltf_json::mesh::Primitive {
		attributes: {
			let mut map = std::collections::BTreeMap::new();
			map.insert(Valid(gltf_json::mesh::Semantic::Positions), positions);
			map.insert(Valid(gltf_json::mesh::Semantic::Normals), normals);
			map.insert(
				Valid(gltf_json::mesh::Semantic::Extras(
					"TERRAIN_WEIGHTS_0_3".into(),
				)),
				weights_0_3,
			);
			map.insert(
				Valid(gltf_json::mesh::Semantic::Extras(
					"TERRAIN_WEIGHTS_4_7".into(),
				)),
				weights_4_7,
			);
			map
		},
		extensions: None,
		extras: None,
		indices: Some(indices),
		material: None,
		mode: Valid(gltf_json::mesh::Mode::Triangles),
		targets: None,
	};

	let mesh = root.push(gltf_json::Mesh {
		extensions: None,
		extras: None,
		name: Some("MountainMeshMesh".into()),
		primitives: vec![primitive],
		weights: None,
	});

	let translation = peak_xform.translation;
	let peak_node = root.push(gltf_json::Node {
		translation: Some([translation.x, translation.z, -translation.y]),
		name: Some("MountainPeak".into()),
		..default()
	});
	let mtn_node = root.push(gltf_json::Node {
		name: Some("MountainMesh".into()),
		mesh: Some(mesh),
		children: Some(vec![peak_node]),
		..default()
	});

	root.push(gltf_json::Scene {
		extensions: None,
		extras: None,
		name: Some("MountainScene".into()),
		nodes: vec![mtn_node],
	});

	let json_string = gltf_json::serialize::to_string(&root).map_err(into_other_io_err)?;
	let glb = Glb {
		header: Header {
			magic: *b"glTF",
			version: 2,
			length: (json_string.len() + buf_len)
				.try_into()
				.map_err(into_other_io_err)?,
		},
		json: Cow::Owned(json_string.into_bytes()),
		bin: Some(Cow::Owned(whole_buffer)),
	};
	let bytes = glb.to_vec().map_err(into_other_io_err)?;

	let mut backup_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
	backup_path.pop();
	backup_path.push("backups");
	backup_path.push(path.file_name().unwrap());
	backup_path.add_extension(format!(
		"bak_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs()
	));

	// Write to a temporary file before replacing old one to help prevent corruption
	let mut tmp_path = path.clone();
	tmp_path.add_extension("tmp~");

	let mut writer = OpenOptions::new()
		.write(true)
		.create(true)
		.truncate(true)
		.open(&tmp_path)
		.await?;
	writer.write_all(&bytes).await?;

	let status = smol::process::Command::new("cp")
		.arg("-n")
		.arg(&path)
		.arg(backup_path)
		.output()
		.await?
		.status;
	if !status.success() {
		panic!("Command did not exit successfully, not saving current scene. Exit code: {status}");
	}

	let status = smol::process::Command::new("mv")
		.arg(&tmp_path)
		.arg(&path)
		.output()
		.await?
		.status;
	if !status.success() {
		error!(
			"Command did not exit successfully, couldn't move temporary file to {path:?}. Exit code: {status}"
		);
	}

	Ok(())
}
