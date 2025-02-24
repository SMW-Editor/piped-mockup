
struct Uniforms {
	resolution: vec2f,
}

@group(0) @binding(0) var<storage> palette: array<vec4f>;
@group(0) @binding(1) var<storage> graphics: array<vec4u>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;

struct VertexIn {
    // Vertex index goes from 0 to 3, for each tile
	@builtin(vertex_index) vertex_index: u32,
	@location(0) tile_instance: vec4u,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let uv = vec2f(vec2u((in.vertex_index << 1) & 2, in.vertex_index & 2)) / 2.0;

    // Actual final position of the vertex
    var position = vec2f(uv);
    position *= 8.0;
    position.x += f32(in.tile_instance.x);
    position.y -= f32(in.tile_instance.y);
    position.y -= 8.0;
    position *= 4.0;
    position.x -= uniforms.resolution.x;
    position.y += uniforms.resolution.y;
    position /= uniforms.resolution;

    return VertexOut(vec4f(position, 0., 1.), uv, in.tile_instance.z, in.tile_instance.w);
}

struct VertexOut {
	@builtin(position) position: vec4f,
	@location(0) uv: vec2f,
	@location(1) tile_id: u32,
	@location(2) pal_scale_flags_flags: u32,
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let uv = vec2u(u32(in.uv.x * 8), u32((1 - in.uv.y) * 8));

    // Since graphics is an array of vec4u, 2 consecutive items in the array make up the bytes for
    // 1 tile.
    let part1 = graphics[in.tile_id * 2 + 0];
    let part2 = graphics[in.tile_id * 2 + 1];

    let lpart1 = part1[uv.y / 2];
    let lpart2 = part2[uv.y / 2];

    let line1 = u32(lpart1 >> ((uv.y & 3) * 16));
    let line2 = u32(lpart2 >> ((uv.y & 3) * 16));

    var color_col: u32 = 0;
    color_col |= ((line1 >> (7 - uv.x)) & 0x1) << 0;
    color_col |= ((line1 >> (15 - uv.x)) & 0x1) << 1;
    color_col |= ((line2 >> (7 - uv.x)) & 0x1) << 2;
    color_col |= ((line2 >> (15 - uv.x)) & 0x1) << 3;
    if color_col == 0 {
		discard;
    } else {
        let pal = in.pal_scale_flags_flags & 0xFF;
        let pal_offset = u32(pal * 0x10);
        return palette[color_col + pal_offset];
    }
}
