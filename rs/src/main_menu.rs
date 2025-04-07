use crate::settings_menu::SettingsMenu;
use crate::state::GlobalState;
use crate::ui::egui::Widget;
use crate::ui::{MenuStack, egui, menu_button};
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use egui::Align2;
use tiny_bail::prelude::r;

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(OnEnter(GlobalState::MainMenu), setup_main_menu)
			.add_systems(
				Update,
				draw_main_menu.run_if(in_state(GlobalState::MainMenu)),
			);
	}
}

pub fn setup_main_menu(mut cmds: Commands) {
	cmds.spawn((Camera2d, StateScoped(GlobalState::MainMenu)));
}

pub fn draw_main_menu(
	mut contexts: EguiContexts,
	mut next_state: ResMut<NextState<GlobalState>>,
	mut exit_events: EventWriter<AppExit>,
	mut menu_stack: ResMut<MenuStack>,
) {
	let ctx = r!(contexts.try_ctx_mut());
	egui::Window::new("Main Menu")
		.anchor(Align2::CENTER_CENTER, [0.0, 0.0])
		.collapsible(false)
		.resizable(false)
		.title_bar(false)
		.show(ctx, |ui| {
			ui.vertical_centered(|ui| {
				let play_btn = menu_button("Play").ui(ui);
				ui.memory_mut(|mem| {
					if mem.focused().is_none() {
						mem.request_focus(play_btn.id);
					}
				});
				if play_btn.clicked() {
					next_state.set(GlobalState::LoadingGame);
				}
				if menu_button("Settings").ui(ui).clicked() {
					menu_stack.push_to_top(SettingsMenu {}).ok();
				}
				if menu_button("Quit").ui(ui).clicked() {
					exit_events.write(AppExit::Success);
				}
			})
		});
}
