// Duotone — map luminance onto a shadow→highlight color ramp.
//   data[0] = shadow.rgba   data[1] = highlight.rgba   data[2] = (amount, _, _, _)
// Input is premultiplied; we operate on straight colour and re-premultiply.
//
// Every plugin shader uses this exact binding group — copy the header verbatim.
struct FxUniforms {
    data:     array<vec4<f32>, 6>,
    texel:    vec2<f32>,
    pass_idx: u32,
    time:     f32,
};

@group(0) @binding(0) var<uniform> u:         FxUniforms;
@group(0) @binding(1) var          t_src:     texture_2d<f32>;
@group(0) @binding(2) var          s_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0)       uv:   vec2<f32>,
}

// Standard full-screen triangle. Copy verbatim.
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((vid << 1u) & 2u);
    let y = f32(vid & 2u);
    out.clip = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    out.uv   = vec2<f32>(x, 1.0 - y);
    return out;
}

const LUMA: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(t_src, s_sampler, in.uv);
    let a = c.a;
    let rgb = select(c.rgb / max(a, 1e-5), vec3<f32>(0.0), a <= 0.0001);

    let l      = dot(rgb, LUMA);
    let duo    = mix(u.data[0].rgb, u.data[1].rgb, l);   // shadow → highlight by luma
    let outRgb = mix(rgb, duo, u.data[2].x);             // amount (blend with original)

    return vec4<f32>(outRgb * a, a);                     // re-premultiply
}
