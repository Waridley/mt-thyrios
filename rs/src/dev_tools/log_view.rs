use crate::ui::egui;
use crate::ui::egui::text::LayoutJob;
use crate::ui::egui::{Align2, Color32};
use bevy::log::{BoxedLayer, tracing_subscriber};
use bevy::prelude::*;
use bevy_console::ConsoleConfiguration;
use bevy_egui::EguiContexts;
use std::io::BufRead;
use std::sync::mpmc;
use std::sync::mpmc::TryRecvError;
use tiny_bail::prelude::r;

pub struct LogViewPlugin;

impl Plugin for LogViewPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(LogViewBuffer::recv_logs, LogViewBuffer::draw).chain(),
		);
	}
}

#[derive(Resource, Debug)]
pub struct LogViewBuffer {
	pub rx: mpmc::Receiver<Vec<u8>>,
	pub buf: String,
	pub layout: LayoutJob,
	pub limit: usize,
}

impl LogViewBuffer {
	pub fn make_layer(app: &mut App) -> Option<BoxedLayer> {
		let (tx, rx) = mpmc::channel();
		app.insert_resource(LogViewBuffer {
			rx,
			buf: default(),
			layout: default(),
			limit: 64_000,
		});

		Some(Box::new(
			tracing_subscriber::fmt::layer()
				.with_target(true)
				.with_ansi(true)
				.with_writer(move || SenderWriter(tx.clone())),
		))
	}

	pub fn recv(
		&mut self,
		max_receives: usize,
		cfg: &ConsoleConfiguration,
	) -> Result<usize, std::io::Error> {
		let mut num_read = 0;
		let mut needs_relayout = false;
		for _ in 0..max_receives {
			let bytes = match self.rx.try_recv() {
				Ok(bytes) => bytes,
				Err(TryRecvError::Empty) => break,
				Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
			};
			needs_relayout = true;
			let s = String::from_utf8_lossy_owned(bytes);
			let mut extra = s.len().saturating_sub(self.limit);
			if extra > 0 {
				while extra < s.len() && !s.is_char_boundary(extra) {
					extra += 1;
				}
				self.buf.clear();
				self.buf.push_str(&s[extra..]);
				num_read += s.len();
				continue;
			}
			let mut extra = (self.buf.len() + s.len()).saturating_sub(self.limit);
			while extra < self.buf.len() && !self.buf.is_char_boundary(extra) {
				extra += 1;
			}
			self.buf.drain(..extra);
			self.buf.push_str(&s);
			num_read += s.len();
		}
		if needs_relayout {
			self.layout = bevy_console::style_ansi_text(&self.buf, cfg);
		}
		Ok(num_read)
	}

	pub fn recv_logs(mut log: ResMut<Self>, cfg: Res<ConsoleConfiguration>) {
		if let Err(e) = log.recv(1024, &cfg) {
			error!("{e}");
		}
	}

	pub fn draw(
		log: Res<Self>,
		mut contexts: EguiContexts,
		keys: Res<ButtonInput<KeyCode>>,
		mut open: Local<bool>,
	) {
		let ctx = r!(contexts.try_ctx_mut());
		if keys.just_pressed(KeyCode::KeyL) {
			*open = !*open;
		}

		egui::Window::new("Logs")
			.open(&mut *open)
			.min_width(720.0)
			.anchor(Align2::LEFT_BOTTOM, [0.0, 0.0])
			.frame(egui::Frame {
				fill: Color32::from_black_alpha(64),
				..default()
			})
			.show(ctx, |ui| {
				egui::ScrollArea::vertical()
					.stick_to_bottom(true)
					.auto_shrink(false)
					.show(ui, |ui| {
						ui.label(log.layout.clone());
					})
			});
	}
}

struct SenderWriter(mpmc::Sender<Vec<u8>>);

impl std::io::Write for SenderWriter {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.0
			.send(buf.into())
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, "failed to send line"))?;
		Ok(buf.len())
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}
