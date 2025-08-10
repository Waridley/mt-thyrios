use crate::ui::egui::{RichText, CornerRadius};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WinitWindows;
pub use bevy_egui::egui;
use bevy_egui::{EguiContext, EguiContextSettings, EguiContexts, EguiPrimaryContextPass, PrimaryEguiContext};
use egui_colors::{Colorix, tokens::ThemeColor};
use std::any::Any;
use std::fmt::Debug;
use bevy::render::camera::Viewport;
use tiny_bail::prelude::r;

pub struct UiPlugin;

pub const DEFAULT_THEME: [ThemeColor; 12] = egui_colors::utils::GRASS_BRONZE;
pub const DEFAULT_SCALE: f32 = 1.0;
pub const DESIGN_HEIGHT: f64 = 1080.0;
pub const SCALE_STEP: f64 = 0.25;

impl Plugin for UiPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<MenuStack>()
			.add_plugins(bevy_egui::EguiPlugin::default())
			.add_systems(PostStartup, setup_egui);
	}
}

pub fn setup_egui(
	mut cmds: Commands,
	// mut contexts: EguiContexts,
	window: Single<Entity, With<PrimaryWindow>>,
	winit_windows: NonSend<WinitWindows>,
) {
	let size = winit_windows
		.get_window(*window)
		.and_then(|window| window.current_monitor())
		.map(|monitor| monitor.size().to_logical::<f64>(monitor.scale_factor()));
	let scale = size
		.map(|size| ((size.height / (DESIGN_HEIGHT * SCALE_STEP)).ceil() * SCALE_STEP) as f32)
		.unwrap_or(DEFAULT_SCALE);
	debug!(scale, ?size);
	// let ctx = r!(contexts.ctx_mut());
	
	// We can't assume the camera exists outside of EguiPrimaryContextPass,
	// but I want to do this exactly once at startup
	let mut ctx = EguiContext::default();
	{
		let ctx = ctx.get_mut();
		ctx.set_theme(egui::Theme::Dark);
		let waridley_theme = [
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x80, 0xff, 0x40]),
			ThemeColor::Custom([0x80, 0xff, 0x40]),
			ThemeColor::Custom([0x80, 0xff, 0x40]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
			ThemeColor::Custom([0x8c, 0x00, 0xff]),
		];
		let colorix = Colorix::global(ctx, DEFAULT_THEME);
		cmds.insert_resource(GameTheme {
			colorix,
			custom_themes: std::iter::once(("Waridley", waridley_theme))
				.chain(
					ALL_SINGLE_COLOR_COLORIX_THEME_NAMES
						.into_iter()
						.zip(ALL_SINGLE_COLOR_COLORIX_THEMES),
				)
				.collect(),
		});
	}
	cmds.spawn((
		Camera2d,
		PrimaryEguiContext,
		ctx,
		EguiContextSettings {
			scale_factor: scale,
			..default()
		},
		// Camera {
		// 	order: 1,
		// 	clear_color: ClearColorConfig::None,
		// 	..default()
		// }
	));
}

#[derive(Resource, Debug)]
pub struct GameTheme {
	pub colorix: Colorix,
	pub custom_themes: (Vec<&'static str>, Vec<[ThemeColor; 12]>),
}

impl GameTheme {
	pub fn color_preset_picker(&mut self, ui: &mut egui::Ui) {
		let custom_themes = self.custom_themes.clone();
		ui.horizontal(|ui| {
			ui.vertical(|ui| {
				// Hack to align dropdown with light/dark toggle button
				// because horizontal_centered expands way too large
				ui.add_space(7.0);
				self.colorix.themes_dropdown(ui, Some(custom_themes), false);
			});
			self.colorix.light_dark_toggle_button(ui, 16.0);
		});
	}
}

const ALL_SINGLE_COLOR_COLORIX_THEMES: [[ThemeColor; 12]; 22] = [
	[ThemeColor::Gray; 12],
	[ThemeColor::EguiBlue; 12],
	[ThemeColor::Tomato; 12],
	[ThemeColor::Red; 12],
	[ThemeColor::Ruby; 12],
	[ThemeColor::Crimson; 12],
	[ThemeColor::Pink; 12],
	[ThemeColor::Plum; 12],
	[ThemeColor::Purple; 12],
	[ThemeColor::Violet; 12],
	[ThemeColor::Iris; 12],
	[ThemeColor::Indigo; 12],
	[ThemeColor::Blue; 12],
	[ThemeColor::Cyan; 12],
	[ThemeColor::Teal; 12],
	[ThemeColor::Jade; 12],
	[ThemeColor::Green; 12],
	[ThemeColor::Grass; 12],
	[ThemeColor::Brown; 12],
	[ThemeColor::Bronze; 12],
	[ThemeColor::Gold; 12],
	[ThemeColor::Orange; 12],
];

const ALL_SINGLE_COLOR_COLORIX_THEME_NAMES: [&str; 22] = [
	"Gray", "EguiBlue", "Tomato", "Red", "Ruby", "Crimson", "Pink", "Plum", "Purple", "Violet",
	"Iris", "Indigo", "Blue", "Cyan", "Teal", "Jade", "Green", "Grass", "Brown", "Bronze", "Gold",
	"Orange",
];

pub fn menu_button(text: impl Into<RichText>) -> egui::Button<'static> {
	egui::Button::new(text.into().strong().size(36.0))
		.min_size([150.0, 50.0].into())
		.corner_radius(CornerRadius::same(20))
}

pub trait Menu: Any + Debug + Send + Sync + 'static {}

impl dyn Menu {
	pub fn is<T: Menu>(&self) -> bool {
		<dyn Any>::is::<T>(self)
	}
	pub fn downcast<T: Menu>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
		if <dyn Any>::is::<T>(&*self) {
			Ok(<Box<dyn Any>>::downcast(self).unwrap())
		} else {
			Err(self)
		}
	}
	pub fn downcast_ref<T: Menu>(&self) -> Option<&T> {
		<dyn Any>::downcast_ref(self)
	}
	pub fn downcast_mut<T: Menu>(&mut self) -> Option<&mut T> {
		<dyn Any>::downcast_mut(self)
	}
}

impl<T: Menu> From<T> for Box<dyn Menu> {
	fn from(value: T) -> Self {
		Box::new(value)
	}
}

#[derive(Resource, Debug, Default)]
pub struct MenuStack {
	stack: Vec<Box<dyn Menu>>,
}

impl MenuStack {
	/// Push a new menu onto the top of the stack if one of the same type is not already somewhere in
	/// the stack. Unlike [push_to_top], this will not bring the existing menu to the top. Instead,
	/// it will only return the provided value.
	pub fn push<T: Menu + Into<Box<dyn Menu>>>(&mut self, menu: T) -> Result<(), T> {
		if self.contains::<T>() {
			Err(menu)
		} else {
			self.stack.push(menu.into());
			Ok(())
		}
	}

	/// Push a new menu onto the top of the stack if one of the same type is not already somewhere
	/// in the stack. If one was already somewhere in the stack, it is brought to the top, and the
	/// provided menu is returned in an `Err`.
	pub fn push_to_top<T: Menu + Into<Box<dyn Menu>>>(&mut self, menu: T) -> Result<(), T> {
		if self.bring_to_top::<T>().is_some() {
			Err(menu)
		} else {
			self.stack.push(menu.into());
			Ok(())
		}
	}

	/// Unconditionally puts a menu on the top of the stack.
	pub fn push_allow_duplicates<T: Into<Box<dyn Menu>>>(&mut self, menu: T) {
		self.stack.push(menu.into());
	}

	/// Brings the topmost menu of type `T` to the top of the stack.
	pub fn bring_to_top<T: Menu>(&mut self) -> Option<&mut T> {
		if let Some(i) = self.stack.iter().rposition(|menu| menu.is::<T>()) {
			let tmp = self.stack.remove(i);
			self.stack.push(tmp);
			Some(self.stack.last_mut().unwrap().downcast_mut::<T>().unwrap())
		} else {
			None
		}
	}

	/// Pop the top menu off the stack if it is of type `T`. Otherwise, the menu stays in place.
	/// If you need to pop the top menu without knowing its type, use [pop_any].
	pub fn pop<T: Menu>(&mut self) -> Option<Box<T>> {
		if self.stack.last().is_some_and(|menu| menu.is::<T>()) {
			Some(<Box<dyn Any>>::downcast(self.stack.pop().unwrap()).unwrap())
		} else {
			None
		}
	}

	/// Unconditionally pops the top of the stack, returning it as a `Box<dyn Menu>` unless the stack
	/// is empty.
	pub fn pop_any(&mut self) -> Option<Box<dyn Menu>> {
		self.stack.pop()
	}

	/// Removes the topmost menu of type `T`.
	pub fn remove_last<T: Menu>(&mut self) -> Option<Box<T>> {
		self.stack.iter().rposition(|menu| menu.is::<T>()).map(|i| self.pull(i).downcast::<T>().unwrap())
	}

	/// Removes the bottommost menu of type `T`.
	pub fn remove_first<T: Menu>(&mut self) -> Option<Box<T>> {
		self.stack.iter().position(|menu| menu.is::<T>()).map(|i| self.pull(i).downcast::<T>().unwrap())
	}

	/// Removes all menus of type `T` from the stack.
	pub fn remove_all<T: Menu>(&mut self) {
		self.stack.retain(|menu| !menu.is::<T>());
	}

	/// Removes menu at index `i` from the stack.
	///
	/// ### Panics
	///
	/// Panics if the length of the stack is less than `i+1`.
	pub fn pull(&mut self, i: usize) -> Box<dyn Menu> {
		self.stack.remove(i)
	}

	/// Downcasts a reference to the top of the stack if it exists and is of type `T`.
	pub fn top<T: Menu>(&self) -> Option<&T> {
		self.stack.last().and_then(|menu| menu.downcast_ref::<T>())
	}

	/// Downcasts a mutable reference to the top of the stack if it exists and is of type `T`.
	pub fn top_mut<T: Menu>(&mut self) -> Option<&mut T> {
		self.stack
			.last_mut()
			.and_then(|menu| menu.downcast_mut::<T>())
	}

	/// Checks whether the top of the stack is of type `T`.
	pub fn top_is<T: Menu>(&self) -> bool {
		self.stack.last().is_some_and(|menu| menu.is::<T>())
	}

	/// Finds the topmost menu of type `T`. To search bottom-up, use [first].
	pub fn last<T: Menu>(&self) -> Option<&T> {
		self.stack
			.iter()
			.rev()
			.find_map(|menu| menu.downcast_ref::<T>())
	}

	/// Finds the bottommost menu of type `T`. To search top-down, use [last].
	pub fn first<T: Menu>(&self) -> Option<&T> {
		self.stack.iter().find_map(|menu| menu.downcast_ref::<T>())
	}

	/// Mutably finds the topmost menu of type `T`. To search bottom-up, use [first_mut].
	pub fn last_mut<T: Menu>(&mut self) -> Option<&mut T> {
		self.stack
			.iter_mut()
			.rev()
			.find_map(|menu| menu.downcast_mut::<T>())
	}

	/// Mutably finds the bottommost menu of type `T`. To search top-down, use [last_mut].
	pub fn first_mut<T: Menu>(&mut self) -> Option<&mut T> {
		self.stack
			.iter_mut()
			.find_map(|menu| menu.downcast_mut::<T>())
	}

	pub fn len(&self) -> usize {
		self.stack.len()
	}
	
	pub fn is_empty(&self) -> bool {
		self.stack.is_empty()
	}

	/// Checks if the stack contains any menu of type `T`.
	pub fn contains<T: Menu>(&self) -> bool {
		self.stack
			.iter()
			.any(|menu| menu.downcast_ref::<T>().is_some())
	}

	/// Iterates through all menus starting at the top of the stack.
	pub fn iter_top_down(&self) -> impl Iterator<Item = &dyn Menu> {
		self.stack.iter().rev().map(|menu| &**menu)
	}

	/// Iterates through all menus mutably, starting at the top of the stack.
	pub fn iter_top_down_mut(&mut self) -> impl Iterator<Item = &mut dyn Menu> {
		self.stack.iter_mut().rev().map(|menu| &mut **menu)
	}

	/// Iterates through all menus, starting at the bottom of the stack.
	pub fn iter_btm_up(&self) -> impl Iterator<Item = &dyn Menu> {
		self.stack.iter().map(|menu| &**menu)
	}

	/// Iterates through all menus mutably, starting at the bottom of the stack.
	pub fn iter_btm_up_mut(&mut self) -> impl Iterator<Item = &mut dyn Menu> {
		self.stack.iter_mut().map(|menu| &mut **menu)
	}
}

pub trait MenuCommandsExt {
	fn push_menu<T: Menu>(&mut self, menu: T);
	fn push_menu_to_top<T: Menu>(&mut self, menu: T);
	fn pop<T: Menu>(&mut self);
	fn pop_any(&mut self);
}

impl MenuCommandsExt for Commands<'_, '_> {
	fn push_menu<T: Menu>(&mut self, menu: T) {
		self.queue(move |world: &mut World| {
			let mut stack = r!(world.get_resource_mut::<MenuStack>());
			r!(stack.push(menu));
		});
	}
	fn push_menu_to_top<T: Menu>(&mut self, menu: T) {
		self.queue(move |world: &mut World| {
			let mut stack = r!(world.get_resource_mut::<MenuStack>());
			r!(stack.push_to_top(menu));
		})
	}
	fn pop<T: Menu>(&mut self) {
		self.queue(move |world: &mut World| {
			let mut stack = r!(world.get_resource_mut::<MenuStack>());
			r!(stack.pop::<T>());
		})
	}
	fn pop_any(&mut self) {
		self.queue(move |world: &mut World| {
			let mut stack = r!(world.get_resource_mut::<MenuStack>());
			stack.pop_any();
		})
	}
}
