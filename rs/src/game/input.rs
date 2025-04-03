use crate::game::cam::{CamAnchor, CamStick, FrameCenter};
use crate::game::mtn::Mountain;
use crate::game::ocean::{OceanSurface, Storm};
use crate::state::GlobalState::InGame;
use bevy::asset::ron;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(InputManagerPlugin::<GameInput>::default())
			.init_resource::<ActionState<GameInput>>()
			.add_systems(Startup, setup_input)
			.add_systems(
				Update,
				(cam_input.run_if(in_state(InGame)), toggle_fullscreen),
			);
	}
}

pub fn setup_input(mut cmds: Commands) {
	let input_map = InputMap::<GameInput>::new([(GameInput::ResetCamPivot, KeyCode::ControlRight)])
		.with_dual_axis(GameInput::MoveCam, VirtualDPad::wasd())
		.with_axis(
			GameInput::Zoom,
			MouseScrollAxis::Y.with_processor(AxisProcessor::Sensitivity(4.0)),
		)
		.with_axis(
			GameInput::Zoom,
			VirtualAxis::new(KeyCode::KeyQ, KeyCode::KeyE),
		)
		.with_dual_axis(GameInput::PivotCam, VirtualDPad::arrow_keys())
		.with_dual_axis(GameInput::PivotCam, VirtualDPad::hjkl());
	debug!(
		"{}",
		ron::ser::to_string_pretty(&input_map, ron::ser::PrettyConfig::default()).unwrap()
	);
	cmds.insert_resource(input_map);
}

pub trait Hjkl {
	fn hjkl() -> Self;
}

impl Hjkl for VirtualDPad {
	fn hjkl() -> Self {
		Self::new(KeyCode::KeyK, KeyCode::KeyJ, KeyCode::KeyH, KeyCode::KeyL)
	}
}

pub fn cam_input(
	mtn: Single<&Mountain>,
	mut anchor: Single<(&mut Transform, &GlobalTransform), With<CamAnchor>>,
	mut frame_center: Single<
		&mut Transform,
		(
			With<FrameCenter>,
			Without<CamAnchor>,
			Without<Camera3d>,
			Without<CamStick>,
		),
	>,
	mut cam: Single<
		&mut Transform,
		(
			With<Camera3d>,
			Without<CamAnchor>,
			Without<FrameCenter>,
			Without<CamStick>,
		),
	>,
	mut stick: Single<
		(&mut Transform, &CamStick),
		(Without<Camera3d>, Without<FrameCenter>, Without<CamAnchor>),
	>,
	ocean_surface: Single<&GlobalTransform, With<OceanSurface>>,
	storm: Res<Storm>,
	state: Res<ActionState<GameInput>>,
	t: Res<Time>,
) {
	let mv = state
		.dual_axis_data(&GameInput::MoveCam)
		.map(|data| data.pair)
		.unwrap_or(Vec2::ZERO);
	let zoom = state
		.axis_data(&GameInput::Zoom)
		.map(|data| data.value)
		.unwrap_or(0.0);
	let input = Vec3::new(mv.x, zoom, mv.y);
	// anchor.translation.z += input.z * t.delta_secs() * 50.0;
	let slope = mtn.slope;
	let dir = Vec3::new(0.0, 1.0, slope).normalize();
	let min_z = ocean_surface.translation().z - anchor.1.translation().z + (storm.intensity * 4.0);
	let min_y = min_z / slope;
	let max_z = f32::max(
		0.0,
		ocean_surface.translation().z - anchor.1.translation().z + (storm.intensity * 4.0),
	);
	frame_center.translation = Vec3::clamp(
		frame_center.translation + input.z * dir * 0.5,
		Vec3::new(0.0, min_y, min_z),
		Vec3::Z * max_z,
	);
	anchor.0.rotation *= Quat::from_rotation_z(input.x * t.delta_secs() * 0.5);

	let dist = cam.translation.length();
	if (input.y < 0.0 && dist < 600.0) || (input.y > 0.0 && dist > 16.0) {
		let forward = -cam.translation.normalize();
		let speed = dist;
		cam.translation = (cam.translation
			+ forward * input.y * f32::min(t.delta_secs(), 0.25) * speed)
			.clamp_length(16.0, 600.0);
	}

	if state.pressed(&GameInput::ResetCamPivot) {
		stick.0.rotation = stick.1.default_rotation;
	} else if let Some(data) = state.dual_axis_data(&GameInput::PivotCam) {
		stick.0.rotation *= Quat::from_rotation_x(-data.pair.y * t.delta_secs());
	}
}

#[derive(Actionlike, Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum GameInput {
	#[actionlike(DualAxis)]
	MoveCam,
	#[actionlike(Axis)]
	Zoom,
	#[actionlike(DualAxis)]
	PivotCam,
	ResetCamPivot,
}

pub fn toggle_fullscreen(
	mut window: Single<&mut Window, With<PrimaryWindow>>,
	keys: Res<ButtonInput<KeyCode>>,
) {
	use WindowMode::*;
	if keys.just_pressed(KeyCode::F11) {
		let new = match window.mode {
			Fullscreen(_, _) => Windowed,
			_ => Fullscreen(MonitorSelection::Current, VideoModeSelection::Current),
		};
		window.mode = new;
	}
}
