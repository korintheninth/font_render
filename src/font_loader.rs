use std::collections::HashMap;

use crate::utils::*;

#[derive(Debug, Clone, Copy)]
pub struct Point {
	pub x: f32,
	pub y: f32,
	pub flags: u8,
}

impl Default for Point {
	fn default() -> Self {
		Self {
			x: 0.0,
			y: 0.0,
			flags: 0,
		}
	}
	
}

#[derive(Debug, Clone)]
pub struct GlyphDescription {
	pub number_of_contours: i16,
	pub xmin: i16,
	pub ymin: i16,
	pub xmax: i16,
	pub ymax: i16,
	pub end_pts_of_contours: Vec<u16>,
	pub num_points: u16,
	pub instruction_length: u16,
	pub instructions: Vec<u8>,
	pub coordinates: Vec<Point>,
}

impl Default for GlyphDescription {
	fn default() -> Self {
		Self {
			number_of_contours: 0,
			xmin: 0,
			ymin: 0,
			xmax: 0,
			ymax: 0,
			end_pts_of_contours: vec![],
			num_points: 0,
			instruction_length: 0,
			instructions: vec![],
			coordinates: vec![],
		}
	}
}

pub struct TableDirectory {
	pub tag: 	String,
	pub offset: u32,
	pub length: u32
}

pub struct FontFile {
	pub file_buffer:				Vec<u8>,
	pub table_directories: 			Vec<TableDirectory>,
	pub glyphs:						Vec<GlyphDescription>,
	pub unicode_to_glyph_index_map: HashMap<u16, u16>
}

impl FontFile {
	pub fn new(file_path: &str) -> FontFile {
		let file_buffer = file_bytes(file_path);
		let table_directories = get_table_details(&file_buffer);
		Self {
			file_buffer,
			table_directories,
			glyphs: vec![],
			unicode_to_glyph_index_map: HashMap::new()
		}
	}
	pub fn get_table_directory(&self, tag: &str) -> &TableDirectory {
		self.table_directories.iter()
			.find(|table| table.tag == tag).unwrap()
	}

	pub fn get_glyphs(&mut self) {
		let num_glyphs_offset = self.get_table_directory("maxp").offset + 4;
		let num_glyphs = get_u16(&self.file_buffer, num_glyphs_offset as usize);

		let byte_entry_check_location = self.get_table_directory("head").offset + 50;
		let entry_size = if get_i16(&self.file_buffer, byte_entry_check_location as usize) == 0 {2} else {4} as u32;

		let location_table_location = self.get_table_directory("loca").offset;
		let glyph_table_location = self.get_table_directory("glyf").offset;

		let mut glyph_locations: Vec<u32> = vec![];
		
		let mut offset = location_table_location;
		for i in 0..num_glyphs as u32{
			let data_offset = if entry_size == 2 {get_u16(&self.file_buffer, offset as usize) as u32 * 2} else {get_u32(&self.file_buffer, offset as usize)};
			glyph_locations.push(glyph_table_location + data_offset);
			offset += entry_size;
		}

		for i in 0..glyph_locations.len() {
			let glyph = self.get_glyph_description(glyph_locations[i] as usize);
			self.glyphs.push(glyph);
		}
		self.insert_inbetween_points();
		//self.insert_bezier_points();
	}

	fn get_glyph_description(&self, mut offset:usize) -> GlyphDescription{
		let mut glyph = GlyphDescription {
			number_of_contours: get_i16(&self.file_buffer, offset),
			xmin: 				get_i16(&self.file_buffer, offset + 2),
			ymin: 				get_i16(&self.file_buffer, offset + 4),
			xmax: 				get_i16(&self.file_buffer, offset + 6),
			ymax: 				get_i16(&self.file_buffer, offset + 8),
			..Default::default()
		};
		offset += 10;
		if glyph.number_of_contours <= 0 {
			return glyph;
		}
		for i in 0..glyph.number_of_contours as usize{
			let point = get_u16(&self.file_buffer, offset);
			glyph.end_pts_of_contours.push(point);
			offset += 2;
		}

		glyph.num_points = glyph.end_pts_of_contours[glyph.end_pts_of_contours.len() - 1] + 1;
		glyph.instruction_length = get_u16(&self.file_buffer, offset);
		offset += 2;
		offset += glyph.instruction_length as usize; // TODO: read instructions

		let points: Vec<Point> = vec![Point::default(); glyph.num_points as usize];
		glyph.coordinates = points;

		let mut flag_count = 0;
		while flag_count < glyph.num_points {
			let flag = self.file_buffer[offset];
			glyph.coordinates[flag_count as usize].flags = flag;
			offset += 1;
			if bit_set(flag, 3){
				let repeat = self.file_buffer[offset];
				offset += 1;
				for i in 0..=repeat as u16{
					glyph.coordinates[(flag_count + i) as usize].flags = flag;
				}
				flag_count += repeat as u16;
			}
			flag_count += 1;
		}

		let mut xcoordinates: Vec<i32> = vec![0; glyph.num_points as usize];
		for i in 0..glyph.num_points as usize {
			xcoordinates[i] = if i > 0 {xcoordinates[i - 1]} else {0};
			
			let flag = glyph.coordinates[i].flags;
			if bit_set(flag, 1) {
        		let dx = self.file_buffer[offset] as i32;
        		xcoordinates[i] += if bit_set(flag, 4) { dx } else { -dx };
				offset += 1;
			}else if !bit_set(flag, 4) {
				xcoordinates[i] += get_i16(&self.file_buffer, offset) as i32;
				offset += 2;
			}
		}

		let mut ycoordinates: Vec<i32> = vec![0; glyph.num_points as usize];
		for i in 0..glyph.num_points as usize {
			ycoordinates[i] = if i > 0 {ycoordinates[i - 1]} else {0};
			
			let flag = glyph.coordinates[i].flags;
			if bit_set(flag, 2) {
				let dy = self.file_buffer[offset] as i32;
				ycoordinates[i] += if bit_set(flag, 5) { dy } else { -dy };
				offset += 1;
			}else if !bit_set(flag, 5) {
				ycoordinates[i] += get_i16(&self.file_buffer, offset) as i32;
				offset += 2;
			}
		}

		for i in 0..glyph.num_points {
			glyph.coordinates[i as usize].x = xcoordinates[i as usize] as f32;
			glyph.coordinates[i as usize].y = ycoordinates[i as usize] as f32;
		}

		glyph
	}

	pub fn get_unicode_to_glyph_index_map(&mut self) {
		let mut offset = self.get_table_directory("cmap").offset as usize;
		let cmap_offset = offset;
		offset += 2;
		let num_tables = get_u16(&self.file_buffer, offset);
		offset += 2;

		struct CmapSubtable {
			platform_id: u16,
			platform_specific_id: u16,
			cmap_offset: u32
		}

		let mut cmap_subtables: Vec<CmapSubtable> = vec![];
		let mut windows_unicode_offset = -1;
		let mut unicode_offset = -1;
		for i in 0..num_tables as usize{
			let platform_id = get_u16(&self.file_buffer, offset);
			offset += 2;
			let platform_specific_id = get_u16(&self.file_buffer, offset);
			offset += 2;
			let cmap_offset = get_u32(&self.file_buffer, offset);
			offset += 4;
			cmap_subtables.push(CmapSubtable{
				platform_id,
				platform_specific_id,
				cmap_offset
			});
		}
		cmap_subtables.sort_by(|a, b| a.platform_id.cmp(&b.platform_id));
		cmap_subtables.sort_by(|a, b| a.platform_specific_id.cmp(&b.platform_specific_id));

		for subtable in cmap_subtables {
			if subtable.platform_id == 3 && subtable.platform_specific_id == 1 {
				windows_unicode_offset = subtable.cmap_offset as i32;
			}else if subtable.platform_id == 0 && subtable.platform_specific_id == 3 {
				unicode_offset = subtable.cmap_offset as i32;
			}
		}

		if windows_unicode_offset == -1 && unicode_offset == -1 {
			panic!("No unicode cmap found");
		}else if windows_unicode_offset != -1 {
			offset = cmap_offset + windows_unicode_offset as usize;
		}else{
			offset = cmap_offset + unicode_offset as usize;
		}

		let format = get_u16(&self.file_buffer, offset);
		offset += 2;
		if format == 4 {
			self.unicode_to_glyph_index_map = self.format_4_cmap(offset)
		} else if format == 12 {
			self.unicode_to_glyph_index_map = self.format_12_cmap()
		} else {
			panic!("Unsupported cmap format: {}", format);
			
		}
	}

	fn format_4_cmap(&self, mut offset: usize) -> HashMap<u16, u16> {
		let length = get_u16(&self.file_buffer, offset);
		
		offset += 4; //skip language
		
		let seg_count = get_u16(&self.file_buffer, offset) / 2;
		
		offset += 8; //skip searchRange, entrySelector, rangeShift

		let mut end_code: Vec<u16> = vec![];
		for _ in 0..seg_count {
			end_code.push(get_u16(&self.file_buffer, offset));
			offset += 2;
		}

		offset += 2; //skip reservedPad
		
		let mut start_code: Vec<u16> = vec![];
		for _ in 0..seg_count {
			start_code.push(get_u16(&self.file_buffer, offset));
			offset += 2;
		}
		
		let mut id_delta: Vec<i16> = vec![];
		for _ in 0..seg_count {
			id_delta.push(get_i16(&self.file_buffer, offset));
			offset += 2;
		}
		
		let id_range_offset_pos = offset;
		let mut id_range_offset: Vec<u16> = vec![];
		for _ in 0..seg_count {
			id_range_offset.push(get_u16(&self.file_buffer, offset));
			offset += 2;
		}
		
		let mut glyph_index_map: HashMap<u16, u16> = HashMap::new();

		for i in 0..seg_count as usize{
			let start = start_code[i];
			let end = end_code[i];
			let delta = id_delta[i];
			let range_offset = id_range_offset[i];
			
			if range_offset == 0 {
				for j in start..=end{
					let index = (j as i16 + delta) as u32 % 65536;
					glyph_index_map.insert(j, index as u16);
				}
			}else{
	            for j in start..=end {
	                let reader_location = id_range_offset_pos + (i * 2);
	                let glyph_index_array_location = 2 * (j - start) as usize + (reader_location + range_offset as usize);
				
	                let glyph_index = get_u16(&self.file_buffer, glyph_index_array_location);
	                let final_index = if glyph_index != 0 {
	                    (glyph_index as i16 + delta) as u32 % 65536
	                } else {
	                    0
	                };
	                glyph_index_map.insert(j, final_index as u16);
	            }
	        }
	    }
	    glyph_index_map
	}

	fn format_12_cmap(&self) -> HashMap<u16, u16> {
		unimplemented!()
	}

	fn insert_inbetween_points(&mut self) {
		for glyph in self.glyphs.iter_mut() {
			let mut new_end_points: Vec<u16> = vec![];
			let mut new_coordinates: Vec<Point> = vec![];
			for i in 0..glyph.end_pts_of_contours.len() {
				let start = if i == 0 {0} else {glyph.end_pts_of_contours[i - 1] + 1};
				let end = glyph.end_pts_of_contours[i];
				for j in start..=end {
					let next_point = glyph.coordinates[if j == end {start} else {j + 1} as usize];
					let current_point = glyph.coordinates[j as usize];
					new_coordinates.push(current_point);
					if bit_set(current_point.flags, 0) == bit_set(next_point.flags, 0) {
						let new_point = Point {
							x: (current_point.x + next_point.x) / 2.0,
							y: (current_point.y + next_point.y) / 2.0,
							flags: !bit_set(current_point.flags, 0) as u8,
						};
						new_coordinates.push(new_point);
					}
				}
				new_end_points.push(new_coordinates.len() as u16 - 1);
			}
			glyph.coordinates = new_coordinates;
			glyph.end_pts_of_contours = new_end_points;
			glyph.num_points = glyph.coordinates.len() as u16;
		}
	}

	fn insert_bezier_points(&mut self) {
		for glyph in self.glyphs.iter_mut() {
			let mut new_end_points: Vec<u16> = vec![];
			let mut new_coordinates: Vec<Point> = vec![];
			for i in 0..glyph.end_pts_of_contours.len() {
				let start = if i == 0 {0} else {glyph.end_pts_of_contours[i - 1] + 1};
				let end = glyph.end_pts_of_contours[i];
				for j in start..=end {
					let start_point = glyph.coordinates[j as usize];
					if !bit_set(start_point.flags, 0) {
						continue;
					}
					let middle_point = glyph.coordinates[if j == end {start} else {j + 1} as usize];
					let end_point = glyph.coordinates[if j == end {start + 1} else if j == end - 1 {start} else {j + 2} as usize];
					let new_points = calculate_beziers(start_point, middle_point, end_point, 25);
					new_coordinates.extend(new_points);
				}
				new_end_points.push(new_coordinates.len() as u16 - 1);
			}
			glyph.coordinates = new_coordinates;
			glyph.end_pts_of_contours = new_end_points;
			glyph.num_points = glyph.coordinates.len() as u16;
		}
	}

	pub fn get_dimensions(&self) -> (i16, i16, i16, i16) {
		let offset = self.get_table_directory("head").offset;
		let xmin_offset = offset as usize + 36;
		let ymin_offset = offset as usize + 38;
		let xmax_offset = offset as usize + 40;
		let ymax_offset = offset as usize + 42;
		(get_i16(&self.file_buffer, xmax_offset), get_i16(&self.file_buffer, ymax_offset), get_i16(&self.file_buffer, xmin_offset), get_i16(&self.file_buffer, ymin_offset))
	}
}

