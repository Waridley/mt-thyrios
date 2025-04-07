use crate::game::mtn::terrain::{StackedTerrainTextures, TexturesMap};
use crate::game::mtn::{Mountain, MountainAssets, MountainScene};
use crate::game::ocean::OceanSurface;
use crate::setup_tracking;
use crate::setup_tracking::{
	IntoDependencyProvider, Progress, RegisterProvider, SetupKey, single_spawn_progress,
	state_progress,
};
use crate::state::GlobalState;
use crate::state::GlobalState::InGame;
use crate::util::set_state_to;
use GlobalState::LoadingGame;
use bevy::app::DynEq;
use bevy::ecs::define_label;
use bevy::ecs::intern::Interned;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::SystemId;
use bevy::ecs::system::lifetimeless::SCommands;
use bevy::prelude::*;
use bevy::scene::SceneInstance;
use bevy::state::state::FreelyMutableState;
use serde::Deserialize;
use setup_tracking::assets_progress;
use std::hash::{Hash, Hasher};
use strum::VariantArray;

pub mod building;
pub mod cam;
pub mod env;
pub mod input;
pub mod mtn;
pub mod ocean;
pub mod pause;
pub mod people;
pub mod research;
pub mod tools;

pub struct GamePlugin;

impl Plugin for GamePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			setup_tracking::SetupTrackingPlugin::<GameSetupKey, _, _, _>::new(
				in_state(LoadingGame),
				set_state_to(InGame),
			),
			building::BuildingPlugin,
			cam::GameCamPlugin,
			env::EnvironmentPlugin,
			input::InputPlugin,
			mtn::MountainPlugin,
			ocean::OceanPlugin,
			pause::PausePlugin,
			tools::ToolsPlugin,
		))
		.add_sub_state::<GameLoadingState>();
	}
}

#[derive(SubStates, Default, Clone, PartialEq, Eq, Hash, Debug)]
#[source(GlobalState = GlobalState::LoadingGame)]
pub enum GameLoadingState {
	#[default]
	LoadingAssets,
	BuildingScene,
	SceneBuilt,
}

define_label!(
	GameSetupLabel,
	GAME_SETUP_LABEL_INTERNER,
	extra_methods: {
		fn register_progress_checker(&self, world: &mut World) -> SystemId<(), Progress>;
		fn relative_time_estimate(&self) -> f32 { 1.0 }
	},
	extra_methods_impl: {
		fn register_progress_checker(&self, world: &mut World) -> SystemId<(), Progress> {
		self.0.register_progress_checker(world)
	}
		fn relative_time_estimate(&self) -> f32 { self.0.relative_time_estimate() }
	}
);

#[macro_export]
macro_rules! new_game_setup_label {
	($T:ident, $progress:expr) => {
		#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
		pub struct $T;

		impl $crate::game::GameSetupLabel for $T {
			fn register_progress_checker(
				&self,
				world: &mut World,
			) -> ::bevy::ecs::system::SystemId<(), $crate::setup_tracking::Progress> {
				world.register_system($progress)
			}

			fn dyn_clone(&self) -> Box<dyn $crate::game::GameSetupLabel> {
				Box::new(self.clone())
			}

			fn as_dyn_eq(&self) -> &dyn ::bevy::ecs::label::DynEq {
				self
			}

			fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
				let ty_id = ::std::any::TypeId::of::<Self>();
				::std::hash::Hash::hash(&ty_id, &mut state);
				::std::hash::Hash::hash(self, &mut state);
			}
		}
	};
}

pub type GameSetupKey = Interned<dyn GameSetupLabel>;

impl SetupKey for Interned<dyn GameSetupLabel> {
	fn register_progress_checker(&self, world: &mut World) -> SystemId<(), Progress> {
		self.0.register_progress_checker(world)
	}

	fn relative_time_estimate(&self) -> f32 {
		self.0.relative_time_estimate()
	}
}

#[cfg(feature = "bevy/track_change_detection")]
pub fn debug_changed_state<S: FreelyMutableState>(next: Option<Res<NextState<S>>>) {
	if let Some(global) = next {
		if global.is_changed() {
			info!(?global, "GlobalState changed: {}", global.changed_by());
		}
	}
}
