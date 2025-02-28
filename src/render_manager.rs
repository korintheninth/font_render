use glutin::{
	self,
	config::ConfigTemplateBuilder,
	context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, Version},
	display::{Display, DisplayApiPreference},
	prelude::*,
	surface::{Surface, WindowSurface},
};
use glow::HasContext;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;
use std::{char, ffi::CString};
use std::fs;
use crate::{font_loader::GlyphDescription, utils::*};
use crate::font_loader::FontFile;


pub struct RenderManager {
	pub gl: glow::Context,
	surface: Surface<WindowSurface>,
	context: glutin::context::PossiblyCurrentContext,
	pub shader_program: glow::Program,
}

impl RenderManager {
	fn load_shader(shader_path: &str) -> String {
		let shader_path = get_global_path(shader_path);
		fs::read_to_string(&shader_path)
			.unwrap_or_else(|_| panic!("Failed to read shader file: {}", shader_path.display()))
	}
	fn compile_shader(gl: &glow::Context, source: &str, shader_type: u32) -> glow::Shader {
		unsafe {
			let shader = gl.create_shader(shader_type).expect("Cannot create shader");
			gl.shader_source(shader, source);
			gl.compile_shader(shader);

			if !gl.get_shader_compile_status(shader) {
				panic!("Failed to compile shader: {}", gl.get_shader_info_log(shader));
			}
			shader
		}
	}

	fn create_shader_program(gl: &glow::Context, vertex_shader: glow::Shader, fragment_shader: glow::Shader) -> glow::Program {
		unsafe {
			let program = gl.create_program().expect("Cannot create program");
			gl.attach_shader(program, vertex_shader);
			gl.attach_shader(program, fragment_shader);
			gl.link_program(program);

			if !gl.get_program_link_status(program) {
				panic!("Failed to link program: {}", gl.get_program_info_log(program));
			}

			gl.delete_shader(vertex_shader);
			gl.delete_shader(fragment_shader);
			program
		}
	}
	pub fn new(window: &Window) -> Self {
		let template = ConfigTemplateBuilder::new()
			.with_alpha_size(8)
			.with_transparency(true)
			.build();

		let display = unsafe {
			Display::new(
				window.display_handle()
					.map_err(|e| e.to_string())
					.unwrap()
					.as_raw(),
				DisplayApiPreference::Wgl(None)
			)
			.expect("Failed to create display")
		};

		let config = unsafe {
			display
				.find_configs(template)
				.expect("Failed to find configs")
				.next()
				.expect("No config found")
		};

			
		let context_attributes = ContextAttributesBuilder::new()
		    .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3)))) 
		    .with_profile(glutin::context::GlProfile::Core)
		    .build(Some(
		        window.window_handle()
		            .map_err(|e| e.to_string())
		            .unwrap()
		            .as_raw(),
		    ));


		let context = unsafe {
			display
				.create_context(&config, &context_attributes)
				.expect("Failed to create context")
		};

		let size = window.inner_size();
		let surface_attributes = 
			glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
				window.window_handle()
					.map_err(|e| e.to_string())
					.unwrap()
					.as_raw(),
				std::num::NonZeroU32::new(size.width).unwrap(),
				std::num::NonZeroU32::new(size.height).unwrap(),
			);

		let surface = unsafe {
			display
				.create_window_surface(&config, &surface_attributes)
				.expect("Failed to create surface")
		};

		let context = context
			.make_current(&surface)
			.expect("Failed to make context current");

		let gl = unsafe {
			glow::Context::from_loader_function(|s| {
				let s = CString::new(s).unwrap();
				display.get_proc_address(s.as_c_str()) as *const _
			})
		};
		
		let vertex_source = Self::load_shader("shaders/vertexshader.vert");
		let fragment_source = Self::load_shader("shaders/fragmentshader.frag");
		
		let vertex_shader = Self::compile_shader(&gl, &vertex_source, glow::VERTEX_SHADER);
		let fragment_shader = Self::compile_shader(&gl, &fragment_source, glow::FRAGMENT_SHADER);
		
		let shader_program = Self::create_shader_program(&gl, vertex_shader, fragment_shader);
		
		Self {
			gl,
			surface,
			context,
			shader_program,
		}
	}

	fn render_outline(&self, size: (u32, u32), text: &str, font_file: &FontFile) {

	    unsafe {
	        self.gl.viewport(0, 0, size.0 as i32, size.1 as i32);
	        self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
			self.gl.line_width(2.0);
	        self.gl.clear(glow::COLOR_BUFFER_BIT);
			
			let mut offset_x = 0;
			let mut offset_y = 0;
			for char in text.chars() {
				if char == '\r' {
					offset_x = 0;
					offset_y -= 1000;
					continue;
				}else if char == ' ' {
					let index = font_file.unicode_to_glyph_index_map.get(&(char as u16)).unwrap(); // change this with data from hmtx table
					let glyph = &font_file.glyphs[*index as usize];
					offset_x += glyph.xmax as i32;
					continue;
				}
				let index = font_file.unicode_to_glyph_index_map.get(&(char as u16)).unwrap();
				let glyph = &font_file.glyphs[*index as usize];
				dbg!(glyph.xmax, glyph.ymax, glyph.xmin, glyph.ymin);

				let indices: Vec<Vec<u32>> = get_indices(glyph);
				
				let scaled_points = scale_points(glyph, size, offset_x, offset_y);
				offset_x += glyph.xmax as i32;

	    		let vbo = create_outline_vbo(&self.gl, scaled_points);
	    		let vao = create_outline_vao(&self.gl, vbo);
				
	    		self.gl.bind_vertex_array(Some(vao));

	    		let ebos = create_outline_ebos(&self.gl, &indices);

	    		self.gl.bind_vertex_array(None);

	        	self.gl.use_program(Some(self.shader_program));
	        	self.gl.bind_vertex_array(Some(vao));

	        	for (ebo, loop_indices) in ebos.iter().zip(indices.iter()) {
	        	    self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(*ebo));
	        	    self.gl.draw_elements(glow::LINE_LOOP, loop_indices.len() as i32, glow::UNSIGNED_INT, 0);
	        	}

	        	self.gl.bind_vertex_array(None);
			}
			
	        self.surface.swap_buffers(&self.context).unwrap();
			check_gl_error(&self.gl, "render loop");
	    }
	}

	fn render_full(&self, size: (u32, u32), text: &str, font_file: &FontFile) {
		let font_dim = font_file.get_dimensions();
		let max_dim = (font_dim.0 - font_dim.2, font_dim.1 - font_dim.3);
		let mut offsets: Vec<(u32, u32)> = vec![];
		let mut glyphs:Vec<GlyphDescription> = vec![];
		let mut next_offset = (0, 0);
		let scale = 0.1;

		for char in text.chars() {
			if char == '\r' {
				next_offset.1 += max_dim.1 as u32;
				next_offset.0 = 0;
			}else if char == ' ' {
				let index = font_file.unicode_to_glyph_index_map.get(&(char as u16)).unwrap(); // change this with data from hmtx table
				let glyph = &font_file.glyphs[*index as usize];
				next_offset.0 += glyph.xmax as u32;
			} else if char == '\t' {
				let index = font_file.unicode_to_glyph_index_map.get(&32).unwrap(); // change this with data from hmtx table
				let glyph = &font_file.glyphs[*index as usize];
				next_offset.0 += glyph.xmax as u32 * 4;
			}
			else {
				let index = font_file.unicode_to_glyph_index_map.get(&(char as u16)).unwrap();
				let glyph = &font_file.glyphs[*index as usize];
				offsets.push(next_offset);
				next_offset.0 += glyph.xmax as u32;
				glyphs.push(glyph.clone());
			}
		}

	    unsafe {
	        self.gl.viewport(0, 0, size.0 as i32, size.1 as i32);
	        self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
			self.gl.line_width(2.0);
	        self.gl.clear(glow::COLOR_BUFFER_BIT);
			
			self.gl.use_program(Some(self.shader_program));

            let resolution_location = self.gl.get_uniform_location(self.shader_program, "u_resolution");
            self.gl.uniform_2_f32(
                resolution_location.as_ref(),
                size.0 as f32,
                size.1 as f32,
            );
            
			let maxdim_location = self.gl.get_uniform_location(self.shader_program, "u_maxdim");
            self.gl.uniform_4_f32(
                maxdim_location.as_ref(),
                font_dim.0 as f32 * scale,
                font_dim.1 as f32 * scale,
				font_dim.2 as f32 * scale,
				font_dim.3 as f32 * scale,
            );
			
			let scale_location = self.gl.get_uniform_location(self.shader_program, "u_scale");
            self.gl.uniform_1_f32(
                scale_location.as_ref(),
            	scale,
            );
			
			if let Some((vao, instance_count)) = create_text_quads_vao(
				&self.gl,
				(100, 100),
				&offsets,
				max_dim,
				size,
				scale
			) {
	        	self.gl.bind_vertex_array(Some(vao));
			
				let (glyph_ssbo, point_ssbo, contour_ssbo) = create_ssbo(&self.gl, &glyphs);
				self.gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(glyph_ssbo));
				self.gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(point_ssbo));
				self.gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 2, Some(contour_ssbo));

	        	self.gl.draw_arrays_instanced(glow::TRIANGLES, 0, 6, instance_count as i32);
				self.gl.bind_vertex_array(None);
			}
	        	
	        self.surface.swap_buffers(&self.context).unwrap();
			check_gl_error(&self.gl, "render loop");
	    }
	}

	pub fn render(&self, size: (u32, u32), text: &str, font_file: &FontFile) {
		self.render_full(size, text, font_file);
		//self.render_outline(size, text, font_file);
	}
}