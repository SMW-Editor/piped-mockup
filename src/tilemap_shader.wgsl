
struct Uniforms {
	resolution: vec2f,
	origin: vec2f,
	scale: f32,
}

@group(0) @binding(0) var<storage> palette: array<vec4f>;
@group(0) @binding(1) var<storage> graphics: array<vec4u>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct VertexIn {
	@builtin(vertex_index) vertex_index: u32,
	@location(0) tile: vec4u,
}

struct VertexOut {
	@builtin(position) position: vec4f,
	@location(0) uv: vec2f,
	@location(1) data: vec2u,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let uv = vec2f(vec2u((in.vertex_index << 1) & 2, in.vertex_index & 2)) / 2.0;
    var position = vec2f(uv);
	position *= 8.0;
	position.x += f32(in.tile.x);
	position.y -= f32(in.tile.y);
	position.y -= 8.0;
	position *= 4.0;
    position.x -= uniforms.resolution.x;
    position.y += uniforms.resolution.y;
	position /= uniforms.resolution;
    return VertexOut(vec4f(position, 0., 1.), uv, in.tile.zw);
}


@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
	let uv = vec2u(u32(in.uv.x * 8), u32((1-in.uv.y) * 8));
	let tile_id = in.data.x;
	let pal_offset = u32((in.data.y & 0xFF) * 0x10);

	let part1 = graphics[tile_id * 2 + 0];
	let part2 = graphics[tile_id * 2 + 1];

	let lpart1 = part1[uv.y / 2];
	let lpart2 = part2[uv.y / 2];

	let line1 = u32(lpart1 >> ((uv.y & 3) * 16));
	let line2 = u32(lpart2 >> ((uv.y & 3) * 16));

	var color_col: u32 = 0;
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
