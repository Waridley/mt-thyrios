use crate::game::cam::CamAnchor;
use crate::state::GlobalState::InGame;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};
use leafwing_input_manager::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(InputManagerPlugin::<GameInput>::default())
			.insert_resource(
				InputMap::<GameInput>::new([(GameInput::ResetCamPivot, KeyCode::ControlRight)])
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
					.with_dual_axis(GameInput::PivotCam, VirtualDPad::hjkl()),
			)
			.init_resource::<ActionState<GameInput>>()
			.add_systems(
				Update,
				(cam_input.run_if(in_state(InGame)), toggle_fullscreen),
			);
	}
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
	mut anchor: Single<&mut Transform, With<CamAnchor>>,
	mut cam: Single<&mut Transform, (With<Camera3d>, Without<CamAnchor>)>,
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
	anchor.translation.z += input.z * t.delta_secs() * 50.0;
	anchor.rotation *= Quat::from_rotation_z(input.x * t.delta_secs());

	let dist = cam.translation.length();
	if (input.y < 0.0 && dist < 400.0) || (input.y > 0.0 && dist > 4.0) {
		let forward = -cam.translation.normalize();
		let speed = dist;
		cam.translation += forward * input.y * t.delta_secs() * speed;
	}

	if state.pressed(&GameInput::ResetCamPivot) {
		cam.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -cam.translation.normalize());
	} else {
		let Some(data) = state.dual_axis_data(&GameInput::PivotCam) else {
			return;
		};
		cam.rotation *= Quat::from_rotation_x(data.pair.y * t.delta_secs());
	}
}

#[derive(Actionlike, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
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
			Fullscreen(_) => Windowed,
			_ => Fullscreen(MonitorSelection::Current),
		};
		window.mode = new;
	}
}
