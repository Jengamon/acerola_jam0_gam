// This shader computes the chromatic aberration effect

// Since post processing is a fullscreen effect, we use the fullscreen vertex shader provided by bevy.
// This will import a vertex shader that renders a single fullscreen triangle.
//
// A fullscreen triangle is a single triangle that covers the entire screen.
// The box in the top left in that diagram is the screen. The 4 x are the corner of the screen
//
// Y axis
//  1 |  x-----x......
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  :´
//    +---------------  X axis
//      -1  0  1  2  3
//
// As you can see, the triangle ends up bigger than the screen.
//
// You don't need to worry about this too much since bevy will compute the correct UVs for you.
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
struct ChromaticAberrationSettings {
    intensity: f32,
	red_offset: vec2<f32>,
	green_offset: vec2<f32>,
	blue_offset: vec2<f32>,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: f32
#endif
}
@group(0) @binding(2) var<uniform> settings: ChromaticAberrationSettings;

fn oob(inp: vec2<f32>) -> bool {
    return inp.x < 0.0 || inp.x > 1.0 || inp.y < 0.0 || inp.y > 1.0;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // Chromatic aberration strength
    // screen center is at (0.5, 0.5)uv, so multiply strength by magnitude of uv to get 0 at center
    // full at edges
    let offset_strength = settings.intensity * length(in.uv - vec2(0.5, 0.5)) * 2.0;
    let red_source   = in.uv + normalize(settings.red_offset) * offset_strength;
    let green_source = in.uv + normalize(settings.green_offset) * offset_strength;
    let blue_source  = in.uv + normalize(settings.blue_offset) * offset_strength;

    var red_pixel   = textureSample(screen_texture, texture_sampler, red_source);
    var green_pixel = textureSample(screen_texture, texture_sampler, green_source);
    var blue_pixel  = textureSample(screen_texture, texture_sampler, blue_source);

    if (oob(red_source)) {
        red_pixel = vec4<f32>(0.0);
    }
    if (oob(green_source)) {
        green_pixel = vec4<f32>(0.0);
    }
    if (oob(blue_source)) {
        blue_pixel = vec4<f32>(0.0);
    }

    // Sample each color channel with an arbitrary shift
    return vec4<f32>(
        red_pixel.r,
        green_pixel.g,
        blue_pixel.b,
        1.0
    );
}
