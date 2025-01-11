use super::mtn::{spawn_mountain, Mountain};
use crate::state::GlobalState::{self, InGame};
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use bevy_atmosphere::plugin::AtmosphereCamera;

pub struct GameCamPlugin;

impl Plugin for GameCamPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(InGame), setup_cam.after(spawn_mountain));
	}
}

pub fn setup_cam(mut cmds: Commands, mountain: Single<Entity, With<Mountain>>) {
	cmds.entity(*mountain).with_children(|mut cmds| {
		cmds.spawn((CamAnchor, Transform::default())).with_child((
			Camera3d::default(),
			Camera {
				clear_color: ClearColorConfig::Custom(Color::BLACK),
				..default()
			},
			// AtmosphereCamera::default(),
			Bloom::default(),
			Transform {
				translation: Vec3::new(0.0, -100.0, 100.0),
				rotation: Quat::from_rotation_arc(
					Vec3::NEG_Z,
					Vec3::new(0.0, 1.0, -1.0).normalize(),
				),
				..default()
			},
			DepthPrepass,
		));
	});
}

#[derive(Component, Debug)]
#[require(StateScoped<GlobalState>(|| StateScoped(InGame)))]
pub struct CamAnchor;
