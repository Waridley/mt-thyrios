use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use crate::game::cam::CamAnchor;
use crate::state::GlobalState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
	fn build(&self, app: &mut App) {
		app
			.add_plugins(InputManagerPlugin::<GameInput>::default())
			.insert_resource(
				InputMap::<GameInput>::default()
					.with_dual_axis(GameInput::MoveCam, VirtualDPad::wasd())
			)
			.init_resource::<ActionState<GameInput>>()
			.add_systems(Update, cam_input.run_if(in_state(GlobalState::InGame)));
	}
}

pub fn cam_input(
	mut cam: Single<&mut Transform, With<CamAnchor>>,
	state: Res<ActionState<GameInput>>,
	t: Res<Time>,
) {
	let Some(data) = state.dual_axis_data(&GameInput::MoveCam) else { return };
	cam.translation.z += data.pair.y * t.delta_secs() * 2.0;
	cam.rotation *= Quat::from_rotation_z(data.pair.x * t.delta_secs());
}

#[derive(Actionlike, Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum GameInput {
	#[actionlike(DualAxis)]
	MoveCam,
}
