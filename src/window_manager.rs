use winit::application::ApplicationHandler;
use winit::event:: WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowId};
use crate::render_manager::{self, RenderManager};
use crate::font_loader::FontFile;
use glow::HasContext;

pub struct App {
	pub window: Option<Window>,
	pub render_manager: Option<RenderManager>,
	pub font_file: Option<FontFile>,
}

impl Default for App {
	fn default() -> Self {
		Self {
			window: None,
			render_manager: None,
			font_file: None,
		}
	}
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let window_attributes = Window::default_attributes()
			.with_title("Font Viewer")
			.with_visible(false);
		let window = event_loop.create_window(window_attributes).unwrap();
		window.set_visible(true);
		window.set_maximized(true);
		self.render_manager = Some(RenderManager::new(&window));
		self.window = Some(window);

		let font_file = self.font_file.as_mut().unwrap();
		font_file.get_glyphs();
		font_file.get_unicode_to_glyph_index_map();
	}
	
	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		static mut RENDER_STRING: String = String::new();

		let size = {
			let window = self.window.as_ref().unwrap();
			let size = window.inner_size();
			(size.width, size.height)
		};
		
		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			},
			WindowEvent::RedrawRequested => {
				if let Some(render_manager) = &self.render_manager {
					render_manager.render(
						size,
						unsafe {RENDER_STRING.as_str()},
						self.font_file.as_ref().unwrap());
				}
			}
			WindowEvent::KeyboardInput {event, ..} => {
				if event.state == winit::event::ElementState::Pressed {
					match event.key_without_modifiers().as_ref() {
						Key::Named(winit::keyboard::NamedKey::Escape) => {
							event_loop.exit();
						},
						_ => (),
					}
					let letter = event.logical_key.to_text();
					if letter.is_some() {
						if letter.unwrap() == "\u{8}" {
							unsafe {RENDER_STRING.pop()};
						} else {
						unsafe {RENDER_STRING.push_str(letter.unwrap())};
						}
						self.window.as_ref().unwrap().request_redraw();
					}
				}
			}
			_ => (),
		}
	}
}