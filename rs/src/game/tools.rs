use bevy::prelude::*;
use std::any::Any;
use std::collections::VecDeque;
use std::fmt::Debug;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<ActiveTool>()
			.add_systems(Last, ActiveTool::cleanup);
	}
}

#[derive(Resource, Default)]
pub struct ActiveTool {
	tool: Option<Box<dyn Tool>>,
	to_cleanup: VecDeque<Box<dyn Tool>>,
}

impl ActiveTool {
	/// Sets the currently active tool. If another one was already active, it will be queued for
	/// [cleanup](Self::cleanup).
	pub fn open(&mut self, tool: impl Into<Box<dyn Tool>>) {
		if let Some(prev) = self.tool.replace(tool.into()) {
			self.to_cleanup.push_back(prev);
		}
	}

	/// Checks if any tool is currently active.
	pub fn any_open(&self) -> bool {
		self.tool.is_some()
	}

	/// Closes the currently active tool, if any, and queues it for cleanup.
	pub fn close_any(&mut self) {
		if let Some(tool) = self.tool.take() {
			self.to_cleanup.push_back(tool);
		}
	}

	/// Closes the currently active tool if it exists and is of type `T`.
	/// Returns `Err(None)` if no tool was active, or `Err(Some(&tool))` if `T` was the wrong type.
	pub fn close<T: Tool>(&mut self) -> Result<(), Option<&dyn Tool>> {
		if self.is::<T>() {
			self.to_cleanup.push_back(self.tool.take().unwrap());
			Ok(())
		} else {
			Err(self.tool.as_ref().map(|tool| &**tool))
		}
	}

	/// Is the currently-active tool of type `T`?
	pub fn is<T: Tool>(&self) -> bool {
		self.tool.as_ref().is_some_and(|tool| tool.is::<T>())
	}

	/// Returns the boxed active `Tool` if it is of type `T`.
	///
	/// **Note:** This bypasses automatic cleanup. Make sure to call [Tool::cleanup] if necessary.
	pub fn try_take_and_bypass_cleanup<T: Tool>(&mut self) -> Option<Box<T>> {
		if self.tool.as_ref().is_some_and(|tool| tool.is::<T>()) {
			Some(self.tool.take().unwrap().downcast::<T>().unwrap())
		} else {
			None
		}
	}

	/// Get a reference to the currently active tool if it exists and is of type `T`.
	pub fn downcast_ref<T: Tool>(&self) -> Option<&T> {
		self.tool.as_ref().and_then(|tool| tool.downcast_ref())
	}

	/// Get a mutable reference to the currently active tool if it exists and is of type `T`.
	pub fn downcast_mut<T: Tool>(&mut self) -> Option<&mut T> {
		self.tool.as_mut().and_then(|tool| tool.downcast_mut::<T>())
	}

	/// System that runs [Tool::cleanup] on all tools that have been removed since it last ran.
	pub fn cleanup(mut this: ResMut<Self>, mut cmds: Commands) {
		let this = this.bypass_change_detection();
		for tool in this.to_cleanup.drain(..) {
			tool.cleanup(&mut cmds);
		}
	}
}

/// A run condition that calls [ActiveTool::is].
pub fn active_tool_is<T: Tool>(active: Res<ActiveTool>) -> bool {
	active.is::<T>()
}

#[reflect_trait]
pub trait Tool: Any + Debug + Send + Sync + 'static {
	/// Optionally override this method to perform any cleanup necessary when this tool is no longer
	/// active.
	fn cleanup(self: Box<Self>, _cmds: &mut Commands) {}
}

impl<T: Tool> From<T> for Box<dyn Tool> {
	fn from(t: T) -> Self {
		Box::new(t)
	}
}

impl dyn Tool {
	pub fn is<T: Tool>(&self) -> bool {
		<dyn Any>::is::<T>(self)
	}
	pub fn downcast<T: Tool>(self: Box<Self>) -> Result<Box<T>, Box<Self>> {
		if <dyn Any>::is::<T>(&*self) {
			Ok(<Box<dyn Any>>::downcast(self).unwrap())
		} else {
			Err(self)
		}
	}
	pub fn downcast_ref<T: Tool>(&self) -> Option<&T> {
		<dyn Any>::downcast_ref::<T>(self)
	}
	pub fn downcast_mut<T: Tool>(&mut self) -> Option<&mut T> {
		<dyn Any>::downcast_mut::<T>(self)
	}
}
