use bevy::asset::RenderAssetUsages;
use super::mtn::{Mountain, MountainHydrated, MountainPeak};
use crate::game::GameSetupLabel;
use crate::new_game_setup_label;
use bird_barrier::{single_spawn_progress, IntoDependencyProvider, RegisterProvider};
use crate::state::GlobalState::{self, InGame};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::experimental::taa::TemporalAntiAliasing;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use bevy::render::camera::{ImageRenderTarget, RenderTarget};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor};
use bevy_egui::PrimaryEguiContext;

new_game_setup_label!(CameraSpawned, single_spawn_progress::<With<Camera3d>>);

pub struct GameCamPlugin;

impl Plugin for GameCamPlugin {
	fn build(&self, app: &mut App) {
		app
			// // If ScreenSpaceReflections get enabled later, uncomment this
			// // and set ocean AlphaBlendMode::Opaque
			// .insert_resource(DefaultOpaqueRendererMethod::deferred())
			.register_provider(
				setup_cam
					.provides([CameraSpawned.key()])
					.requires([MountainHydrated.key()]),
			)
			.add_systems(OnEnter(InGame), activate_cam);
	}
}

pub fn setup_cam(
	mut cmds: Commands,
	peak: Single<Entity, With<MountainPeak>>,
	mtn: Single<&Mountain>,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<ColorMaterial>>,
) {
	let slope = mtn.slope;
	let anchor_z = slope * mtn.plateau_radius;

	// Workaround for https://github.com/bevyengine/bevy/issues/12121
	// Might want to keep a separate render target anyway for graphics quality options, but will
	// need a better way to dynamically adjust it to window size and user preference.
	let mut render_target = Image::new_fill(
		Extent3d {
			width: 2560,
			height: 1440,
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		&[0; 4],
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::all(),
	);
	render_target.texture_descriptor.usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC | TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
	let render_target = images.add(render_target);
	let _view_3d = cmds.spawn((
		Mesh2d(meshes.add(Rectangle::new(2560.0, 1440.0).mesh())),
		MeshMaterial2d(mats.add(ColorMaterial {
			texture: Some(render_target.clone()),
			..default()
		})),
		StateScoped(InGame),
	));
	let render_target = RenderTarget::Image(render_target.into());
	
	cmds.entity(*peak).with_children(|cmds| {
		cmds.spawn((CamAnchor, Transform::from_translation(Vec3::Z * anchor_z)))
			.with_children(|cmds| {
				let stick_z = (-mtn.peak_elevation * 0.8) - anchor_z;
				cmds.spawn((
					FrameCenter,
					Transform::from_translation(Vec3::new(0.0, stick_z / slope, stick_z)),
				))
				.with_children(|cmds| {
					let rot =
						Quat::from_rotation_arc(Vec3::Z, Vec3::new(0.0, -3.0, 1.0).normalize());
					cmds.spawn((
						CamStick {
							default_rotation: rot,
						},
						Transform::from_rotation(rot),
					))
					.with_child((
						Camera3d::default(),
						Camera {
							clear_color: ClearColorConfig::Custom(Color::BLACK),
							is_active: false,
							hdr: true,
							target: render_target,
							..default()
						},
						Projection::Perspective(PerspectiveProjection {
							fov: std::f32::consts::FRAC_PI_8,
							..default()
						}),
						Bloom::default(),
						Transform::from_translation(Vec3::new(0.0, 0.0, 150.0)),
						DepthPrepass,
						TemporalAntiAliasing::default(),
						Msaa::Off,
						// // Maybe later
						// //
						// // Be sure to set DefaultOpaqueRendererMethod::deferred()
						// // and set ocean AlphaBlendMode::Opaque if this is enabled.
						// //
						// // This would require a custom ocean render pass to enable shallow water
						// // transparency, but we might want to do that anyway to fix the render order for
						// // triangles behind the mountain.
						// //
						// ScreenSpaceReflections {
						// 	thickness: 0.0625,
						// 	linear_march_exponent: 2.0,
						// 	..default()
						// },
					));
					cmds.spawn((
						CamLightStick,
						Transform::from_rotation(Quat::from_rotation_arc(
							Vec3::NEG_Z,
							Vec3::new(0.0, slope, -1.0).normalize(),
						)),
					))
					.with_child((
						CamLight,
						SpotLight {
							intensity: 50_000_000.0,
							range: 300.0,
							outer_angle: std::f32::consts::FRAC_PI_4,
							..default()
						},
						Transform {
							translation: Vec3::new(0.0, 0.0, 100.0),
							..default()
						},
					));
				});
			});
	});
}

pub fn activate_cam(mut cam: Single<&mut Camera, With<Camera3d>>) {
	cam.is_active = true;
}

#[derive(Component, Debug)]
#[require(Transform, Visibility, StateScoped<GlobalState> = StateScoped(InGame))]
pub struct CamAnchor;

#[derive(Component, Debug)]
#[require(Transform, Visibility, StateScoped<GlobalState> = StateScoped(InGame))]
pub struct FrameCenter;

#[derive(Component, Debug)]
#[require(Transform, Visibility, StateScoped<GlobalState> = StateScoped(InGame))]
pub struct CamStick {
	pub default_rotation: Quat,
}

#[derive(Component, Debug)]
#[require(Transform, Visibility, StateScoped<GlobalState> = StateScoped(InGame))]
pub struct CamLightStick;

#[derive(Component, Debug)]
#[require(Transform, Visibility, StateScoped<GlobalState> = StateScoped(InGame))]
pub struct CamLight;
