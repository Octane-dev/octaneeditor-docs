# Recipes

## A one-param tint (Track A)

Folder `redshift/`:

`manifest.ron`
```ron
(
    id: "redshift", name: "Red Shift", category: "color", icon: "tint",
    params: [ (id: "amt", label: "Amount", kind: Percent(default: 0.5, min: 0.0, max: 1.0)) ],
    passes: [ (src: 0, dst: 1) ],
)
```

`effect.wgsl` (header + `vs_main` from the [shader contract](./track-a.md#effectwgsl--the-shader-contract), then):
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(t_src, s_sampler, in.uv);
    let a = c.a;
    let rgb = select(c.rgb / max(a, 1e-5), vec3<f32>(0.0), a <= 0.0001);
    let shifted = mix(rgb, vec3<f32>(rgb.r, rgb.g * 0.5, rgb.b * 0.3), u.data[0].x);
    return vec4<f32>(shifted * a, a);
}
```

## An animated generator (Track A)

Use `u.time` (seconds) and `in.uv`:
```wgsl
let wave = 0.5 + 0.5 * sin(in.uv.x * 20.0 + u.time * 3.0);
return vec4<f32>(vec3<f32>(wave), 1.0);
```

## A separable blur (Track A, multi-pass)

```ron
passes:  [ (src: 0, dst: 1), (src: 1, dst: 0) ],   // horizontal then vertical
buffers: 2,
```
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // u.data[0].x = radius in px; branch on u.pass_idx for the axis
    let dir = select(vec2<f32>(0.0, u.texel.y), vec2<f32>(u.texel.x, 0.0), u.pass_idx == 0u);
    var acc = vec4<f32>(0.0);
    for (var i = -4; i <= 4; i = i + 1) {
        acc = acc + textureSample(t_src, s_sampler, in.uv + dir * f32(i) * u.data[0].x);
    }
    return acc / 9.0;
}
```

## "Distribute selection evenly" command (Track W)

```rust
#[no_mangle]
pub extern "C" fn distribute_x() {
    let n = octane::selected_count();
    if n < 2 { return; }
    let margin = 120.0;
    let span = octane::comp_width() - margin * 2.0;
    for i in 0..n {
        let id = octane::selected_id(i);
        let x  = margin + span * (i as f32 / (n - 1) as f32);
        octane::set_position(id, x, octane::layer_y(id));
    }
}
```

## A minimal importer (Track W)

```rust
#[no_mangle]
pub extern "C" fn octane_register() {
    octane::register_importer("Plain Text (.txt)", &["txt"], "import_txt");
}

#[no_mangle]
pub extern "C" fn import_txt() {
    let bytes = octane::input_bytes();
    let text = String::from_utf8_lossy(&bytes).to_string();
    octane::create_comp("Text Import", 1920, 1080, 30.0, 150.0);
    octane::add_text(&text, 960.0, 540.0, 72.0);
}
```
