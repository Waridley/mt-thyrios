use crate::settings_menu::SettingsMenu;
use crate::state::GlobalState;
use crate::ui::egui::text::LayoutJob;
use crate::ui::egui::{Color32, Rounding};
use crate::ui::{Menu, MenuStack, menu_button};
use bevy::prelude::*;
use bevy_egui::egui::text::LayoutSection;
use bevy_egui::egui::{Align, Align2, Margin, RichText, Stroke, Widget, WidgetText};
use bevy_egui::{EguiContexts, egui};
use bevy_steamworks::Input;

pub struct PausePlugin;

impl Plugin for PausePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				PauseMenu::draw,
				PauseMenu::toggle_on_escape.run_if(in_state(GlobalState::InGame)),
			),
		);
	}
}

#[derive(Debug, Reflect)]
pub struct PauseMenu {}

impl Menu for PauseMenu {}

/// A run condition that returns true if the game is paused.
pub fn paused(t: Res<Time<Virtual>>) -> bool {
	t.is_paused()
}

/// A run condition that returns true if the game is unpaused.
pub fn unpaused(t: Res<Time<Virtual>>) -> bool {
	!t.is_paused()
}

impl PauseMenu {
	pub fn toggle_on_escape(
		mut cmds: Commands,
		keys: Res<ButtonInput<KeyCode>>,
		t: Res<Time<Virtual>>,
	) {
		if keys.just_pressed(KeyCode::Escape) {
			if t.is_paused() {
				cmds.unpause();
			} else {
				cmds.pause();
			}
		}
	}

	pub fn draw(
		mut cmds: Commands,
		mut contexts: EguiContexts,
		mut next_state: ResMut<NextState<GlobalState>>,
		mut exit_events: EventWriter<AppExit>,
		mut menu_stack: ResMut<MenuStack>,
	) {
		let ctx = contexts.ctx_mut();

		let was_open = menu_stack.contains::<Self>();
		let mut open = was_open;
		let is_top = menu_stack.top_is::<Self>();
		egui::Window::new("Pause")
			.anchor(Align2::CENTER_CENTER, [0.0, 0.0])
			.enabled(is_top)
			.interactable(is_top)
			.resizable(false)
			.collapsible(false)
			.open(&mut open)
			.show(ctx, |ui| {
				ui.vertical_centered(|ui| {
					if menu_button("Resume").ui(ui).clicked() {
						cmds.unpause();
					}
					if menu_button("Main Menu").ui(ui).clicked() {
						next_state.set(GlobalState::MainMenu);
						cmds.unpause();
					}
					if menu_button("Settings").ui(ui).clicked() {
						menu_stack.push_to_top(SettingsMenu {}).ok();
					}
					if menu_button("Quit").ui(ui).clicked() {
						exit_events.send(AppExit::Success);
					}
				})
			});
		if was_open && !open {
			cmds.unpause();
		}
	}
}

pub trait PauseGame {
	fn pause(&mut self);
	fn unpause(&mut self);
}

impl PauseGame for Commands<'_, '_> {
	fn pause(&mut self) {
		self.queue(|world: &mut World| {
			if let Err(_) = world.resource_mut::<MenuStack>().push(PauseMenu {}) {
				warn!("Game was already paused");
			}
			world.resource_mut::<Time<Virtual>>().pause();
		});
	}

	fn unpause(&mut self) {
		self.queue(|world: &mut World| {
			if world
				.resource_mut::<MenuStack>()
				.pop::<PauseMenu>()
				.is_none()
			{
				error!("Top of MenuStack was not PauseMenu");
			}
			world.resource_mut::<Time<Virtual>>().unpause();
		})
	}
}
