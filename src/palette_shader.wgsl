@group(0) @binding(0) var<storage> palette: array<vec4f>;

struct VertexIn {
	@builtin(vertex_index) vertex_index: u32,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let uv = vec2f(vec2u((in.vertex_index << 1) & 2, in.vertex_index & 2));
    var position = vec2f(uv * 2. - 1.);
    return VertexOut(vec4f(position, 0., 1.), uv);
}

struct VertexOut {
	@builtin(position) position: vec4f,
	@location(0) uv: vec2f,
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4f {
    let uv = vec2u(u32(in.uv.x * 16), u32((1 - in.uv.y) * 16));
    let idx = uv.x + uv.y * 16;
    return palette[idx];
}
