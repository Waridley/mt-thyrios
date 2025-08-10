use crate::ui::{GameTheme, Menu, MenuStack, egui};
use bevy::prelude::*;
use bevy_egui::{EguiContextSettings, EguiContexts, EguiPrimaryContextPass};
use egui::{Align2, Widget};
use tiny_bail::prelude::r;

pub struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(EguiPrimaryContextPass, SettingsMenu::draw);
	}
}

#[derive(Debug)]
pub struct SettingsMenu {}

impl Menu for SettingsMenu {}

impl SettingsMenu {
	pub fn draw(
		mut contexts: EguiContexts,
		mut theme: Option<ResMut<GameTheme>>,
		mut egui_settings: Single<&mut EguiContextSettings>,
		mut zoom: Local<f32>,
		mut menu_stack: ResMut<MenuStack>,
	) {
		if *zoom < 0.5 {
			*zoom = egui_settings.scale_factor;
		}
		let ctx = r!(contexts.ctx_mut());

		let was_open = menu_stack.contains::<Self>();
		let mut open = was_open;
		let is_top = menu_stack.top_is::<Self>();
		egui::Window::new("Settings")
			.anchor(Align2::CENTER_CENTER, [0.0, 0.0])
			.enabled(is_top)
			.interactable(is_top)
			.movable(false)
			.resizable(false)
			.collapsible(false)
			.open(&mut open)
			.show(ctx, |ui| {
				ui.heading("UI Theme");
				ui.add_space(5.0);
				if let Some(mut theme) = theme {
					theme.color_preset_picker(ui);
				}
				ui.add_space(5.0);
				ui.heading("UI Scale");
				egui::Slider::new(&mut *zoom, 0.5..=8.0)
					.step_by(crate::ui::SCALE_STEP)
					.logarithmic(true)
					.ui(ui);
				ui.add_space(5.0);
				let apply_enabled = egui_settings.scale_factor != *zoom;
				ui.horizontal(|ui| {
					// TODO: Add a revert timer popup
					if ui
						.add_enabled(apply_enabled, egui::Button::new("Cancel"))
						.clicked()
					{
						*zoom = egui_settings.scale_factor;
					}
					if ui
						.add_enabled(apply_enabled, egui::Button::new("Apply"))
						.clicked()
					{
						egui_settings.scale_factor = *zoom;
					}
				});
			});
		if was_open && !open {
			*zoom = egui_settings.scale_factor;
			if menu_stack.pop::<SettingsMenu>().is_none() {
				error!("Top of MenuStack wasn't SettingsMenu");
			}
		}
	}
}
