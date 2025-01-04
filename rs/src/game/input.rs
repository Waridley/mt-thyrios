use std::f32::consts::FRAC_1_SQRT_2;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use crate::state::GlobalState::InGame;
use crate::game::cam::CamAnchor;

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_plugins(InputManagerPlugin::<GameInput>::default())
			.insert_resource(
				InputMap::<GameInput>::new([
					(GameInput::ResetCamPivot, KeyCode::ControlRight),
				])
					.with_triple_axis(GameInput::MoveCam, VirtualDPad3D::new(
						KeyCode::KeyE,
						KeyCode::KeyQ,
						KeyCode::KeyA,
						KeyCode::KeyD,
						KeyCode::KeyW,
						KeyCode::KeyS,
					))
					.with_dual_axis(GameInput::PivotCam, VirtualDPad::arrow_keys())
			)
			.init_resource::<ActionState<GameInput>>()
			.add_systems(Update, cam_input.run_if(in_state(InGame)));
	}
}

pub fn cam_input(
	mut anchor: Single<&mut Transform, With<CamAnchor>>,
	mut cam: Single<&mut Transform, (With<Camera3d>, Without<CamAnchor>)>,
	state: Res<ActionState<GameInput>>,
	t: Res<Time>,
) {
	let Some(data) = state.triple_axis_data(&GameInput::MoveCam) else { return };
	let input = Vec3::new(data.triple.x, -data.triple.z, data.triple.y);
	anchor.translation.z += input.z * t.delta_secs() * 3.0;
	anchor.rotation *= Quat::from_rotation_z(input.x * t.delta_secs());
	
	let dist = cam.translation.length();
	if (input.y < 0.0 && dist < 40.0) || (input.y > 0.0 && dist > 1.0) {
		let forward = -cam.translation.normalize();
		let speed = dist * 0.5;
		cam.translation += forward * input.y * t.delta_secs() * speed;
	}
	
	if state.pressed(&GameInput::ResetCamPivot) {
		cam.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -cam.translation.normalize());
	} else {
		let Some(data) = state.dual_axis_data(&GameInput::PivotCam) else { return };
		cam.rotation *= Quat::from_rotation_x(data.pair.y * t.delta_secs());
	}
}

#[derive(Actionlike, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum GameInput {
	#[actionlike(TripleAxis)]
	MoveCam,
	#[actionlike(DualAxis)]
	PivotCam,
	ResetCamPivot,
}
