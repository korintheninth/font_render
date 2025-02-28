use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::str::{self, FromStr};
use glow::*;
use crate::font_loader::{GlyphDescription, Point, TableDirectory};

pub fn get_u32(buffer: &Vec<u8>, offset: usize) -> u32 {
	u32::from_be_bytes(buffer[offset..offset + 4].try_into().unwrap())
}

pub fn get_u16(buffer: &Vec<u8>, offset: usize) -> u16 {
	u16::from_be_bytes(buffer[offset..offset + 2].try_into().unwrap())
}

pub fn get_i16(buffer: &Vec<u8>, offset: usize) -> i16 {
	i16::from_be_bytes(buffer[offset..offset + 2].try_into().unwrap())
}

pub fn bit_set(byte: u8, bit: u8) -> bool {
	((byte >> bit) & 1) != 0
}

pub fn get_global_path(relative_path: &str) -> PathBuf {
    let base_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    } else {
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    };
    
    return base_dir.join(relative_path)
}

pub fn file_bytes(file_path: &str) -> Vec<u8> {
	let global_file_path = get_global_path(file_path);
	let mut file = File::open(global_file_path.to_str().unwrap()).unwrap();
	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer).unwrap();
	buffer
}

pub fn get_num_tables(buffer: &Vec<u8>) -> u16 {
	let num_tables = get_u16(&buffer, 4);
	num_tables
}

pub fn get_table_details(buffer: &Vec<u8>) -> Vec<TableDirectory>{
	let num_tables = get_num_tables(&buffer);
	let mut tables: Vec<TableDirectory> = vec![];

	for i in 0..num_tables{
		let tag_bytes = str::from_utf8(&buffer[(12 + i * 16) as usize..(16 + i * 16) as usize]).unwrap();
		let offset_bytes = get_u32(buffer, (20 + i * 16) as usize);
		let length_bytes = get_u32(buffer, (24 + i * 16) as usize);

		let table= TableDirectory{
			tag: String::from_str(tag_bytes).unwrap(),
			offset: offset_bytes,
			length: length_bytes
		};
		tables.push(table);
	}
	tables
}

pub fn get_indices(glyph: &GlyphDescription) -> Vec<Vec<u32>> {
	let mut indices: Vec<Vec<u32>> = vec![];
	let mut prev_end = 0;
	for &end in &glyph.end_pts_of_contours {
	    let current_indices = (prev_end..=end).map(|j| j as u32).collect();
	    indices.push(current_indices);
	    prev_end = end + 1;
	}
	indices
}

pub fn scale_points(glyph: &GlyphDescription, size: (u32, u32), offset_x: i32, offset_y: i32) -> Vec<(f32, f32)> {
	let points = &glyph.coordinates;
	let xmax = glyph.xmax as i16;
	let ymax = glyph.ymax as i16;

	let scale_factor_x = xmax as f32 / (size.0 * 10) as f32;
	let scale_factor_y = ymax as f32 / (size.1 * 10) as f32;
	let scaled_points: Vec<(f32, f32)> = points.iter().map(|point| {
		(((point.x + offset_x as f32) / xmax as f32) * scale_factor_x as f32 - 1.0, ((point.y + offset_y as f32) / ymax as f32) * scale_factor_y as f32)
	}).collect();
	
	scaled_points
}

pub fn create_outline_vbo(gl: &Context, points: Vec<(f32, f32)>) -> NativeBuffer {
    let flattened: Vec<f32> = points.iter().flat_map(|&(x, y)| vec![x, y]).collect();

    unsafe {
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&flattened), glow::STATIC_DRAW);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        vbo
    }
}

pub fn create_outline_ebos(gl: &Context, indices: &Vec<Vec<u32>>) -> Vec<NativeBuffer> {

    indices.iter().map(|loop_indices| {
        unsafe {
            let ebo = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytemuck::cast_slice(loop_indices), glow::STATIC_DRAW);
            ebo
        }
    }).collect()
}

pub fn create_outline_vao(gl: &Context, vbo: NativeBuffer) -> NativeVertexArray {
    unsafe {
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        
		gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, 0);
        gl.enable_vertex_attrib_array(0);

        gl.bind_vertex_array(None);
        vao
    }
}

pub fn check_gl_error(gl: &Context, operation: &str) {
    unsafe {
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            println!("OpenGL error after {}: 0x{:X}", operation, error);
        }
    }
}

pub fn calculate_beziers(start_point: Point, middle_point: Point, end_point: Point, resolution: i32) -> Vec<Point> {
	let mut points: Vec<Point> = vec![];
	for i in 0..resolution {
		let t = i as f32 / resolution as f32;
		let x = (1.0 - t).powi(2) * start_point.x + 2.0 * (1.0 - t) * t * middle_point.x + t.powi(2) * end_point.x;
		let y = (1.0 - t).powi(2) * start_point.y + 2.0 * (1.0 - t) * t * middle_point.y + t.powi(2) * end_point.y;
		points.push(Point { x, y, flags: 1});
	}
	points
}

pub fn create_text_quads_vao(
    gl: &Context,
    position: (u32, u32),
    offsets: &Vec<(u32, u32)>,
    size: (i16, i16),
    viewport_size: (u32, u32),
    scale: f32
) -> Option<(NativeVertexArray, usize)> {
    if offsets.is_empty() {
        return None;
    }
    
    let normalize_x = |x: u32| (2.0 * x as f32 / viewport_size.0 as f32) - 1.0;
    let normalize_y = |y: u32| 1.0 - (2.0 * y as f32 / viewport_size.1 as f32);
    
    let scaled_width = (size.0 as f32 * scale) as u32;
    let scaled_height = (size.1 as f32 * scale) as u32;
    
    let vertices = vec![
        0.0, 0.0,
        0.0, scaled_height as f32,
        scaled_width as f32, 0.0,
        
        0.0, scaled_height as f32,
        scaled_width as f32, scaled_height as f32,
        scaled_width as f32, 0.0,
    ];
    
    let instance_data = offsets.iter().map(|&(x, y)| {
        let scaled_offset_x = (x as f32 * scale) as u32;
        let scaled_offset_y = (y as f32 * scale) as u32;
        
        let x_pos = position.0 + scaled_offset_x;
        let y_pos = position.1 + scaled_offset_y;
        
        let x_normalized = normalize_x(x_pos);
        let y_normalized = normalize_y(y_pos);
        
        [x_normalized, y_normalized]
    }).flatten().collect::<Vec<f32>>();
    
    unsafe {
        let vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER, 
            bytemuck::cast_slice(&vertices), 
            glow::STATIC_DRAW
        );
        
        let instance_vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&instance_data),
            glow::STATIC_DRAW
        );
        
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));
        
        // Bind the vertex buffer for the quad vertices
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.vertex_attrib_pointer_f32(
            0, 
            2,
            glow::FLOAT, 
            false, 
            2 * std::mem::size_of::<f32>() as i32, 
            0
        );
        gl.enable_vertex_attrib_array(0);
        
        // Bind the instance data buffer for positions
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_vbo));
        gl.vertex_attrib_pointer_f32(
            1, 
            2,
            glow::FLOAT, 
            false, 
            2 * std::mem::size_of::<f32>() as i32, 
            0
        );
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_divisor(1, 1);  // This makes it instanced
        
        gl.bind_vertex_array(None);
        
        // Return the VAO and the instance count
        Some((vao, offsets.len()))
    }
}

pub fn create_ssbo(gl: &Context, glyphs: &Vec<GlyphDescription>) -> (NativeBuffer, NativeBuffer, NativeBuffer) {
	let mut glyph_data = vec![];
	let mut point_data = vec![];
	let mut contour_data = vec![];

	let mut point_offset = 0;
	let mut contour_offset = 0;

	for glyph in glyphs {
	    glyph_data.push(glyph.xmin as i32);
	    glyph_data.push(glyph.ymin as i32);
	    glyph_data.push(glyph.xmax as i32);
	    glyph_data.push(glyph.ymax as i32);
	    glyph_data.push(glyph.coordinates.len() as i32);
	    glyph_data.push(glyph.end_pts_of_contours.len() as i32);
	    glyph_data.push(point_offset);
	    glyph_data.push(contour_offset);

	    for point in &glyph.coordinates {
	        point_data.push(point.x as i32);
	        point_data.push(point.y as i32);
	        point_data.push(point.flags as i32);
	    }
	    point_offset += glyph.coordinates.len() as i32;

	    for &contour in &glyph.end_pts_of_contours {
	        contour_data.push(contour as i32);
	    }
	    contour_offset += glyph.end_pts_of_contours.len() as i32;
	}

	unsafe {
	    let glyph_ssbo = gl.create_buffer().unwrap();
	    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(glyph_ssbo));
	    gl.buffer_data_u8_slice(
	        glow::SHADER_STORAGE_BUFFER,
	        bytemuck::cast_slice(&glyph_data),
	        glow::STATIC_DRAW,
	    );

	    let point_ssbo = gl.create_buffer().unwrap();
	    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(point_ssbo));
	    gl.buffer_data_u8_slice(
	        glow::SHADER_STORAGE_BUFFER,
	        bytemuck::cast_slice(&point_data),
	        glow::STATIC_DRAW,
	    );

	    let contour_ssbo = gl.create_buffer().unwrap();
	    gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(contour_ssbo));
	    gl.buffer_data_u8_slice(
	        glow::SHADER_STORAGE_BUFFER,
	        bytemuck::cast_slice(&contour_data),
	        glow::STATIC_DRAW,
	    );
		(glyph_ssbo, point_ssbo, contour_ssbo)
	}
}