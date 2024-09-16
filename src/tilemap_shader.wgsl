/*
struct Uniforms {
	resolution: vec2f,
	center: vec2f,
	scale: f32,
	max_iter: u32,
}

@group(0) @binding(0) var<storage> uniforms: Uniforms;*/

@group(0) @binding(0) var<storage> palette: array<vec4f>;
@group(0) @binding(1) var<storage> graphics: array<vec4u>;

struct VertexIn {
	@builtin(vertex_index) vertex_index: u32,
}

struct VertexOut {
	@builtin(position) position: vec4f,
	@location(0) uv: vec2f,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let uv = vec2f(vec2u((in.vertex_index << 1) & 2, in.vertex_index & 2));
    let position = vec4f(uv * 2. - 1., 0., 1.);
    return VertexOut(position, uv);
}


@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
	let uv = vec2u(u32(in.uv.x * 8), u32((1-in.uv.y) * 8));
	let tile_id = 0x14;
	let pal_offset = 0x50;

	let part1 = graphics[tile_id * 2 + 0];
	let part2 = graphics[tile_id * 2 + 1];

	let lpart1 = part1[uv.y / 2];
	let lpart2 = part2[uv.y / 2];

	let line1 = i32(lpart1 >> ((uv.y & 3) * 16));
	let line2 = i32(lpart2 >> ((uv.y & 3) * 16));

	var color_col = 0;
	color_col |= ((line1 >> ( 7 - uv.x)) & 0x1) << 0;
	color_col |= ((line1 >> (15 - uv.x)) & 0x1) << 1;
	color_col |= ((line2 >> ( 7 - uv.x)) & 0x1) << 2;
	color_col |= ((line2 >> (15 - uv.x)) & 0x1) << 3;
	if color_col == 0 {
		discard;
	} else {
		return palette[color_col+pal_offset];
	}
}
