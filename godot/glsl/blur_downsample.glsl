/* clang-format off */
#[vertex]

#version 450

layout(location = 0) out vec2 uv_interp;
/* clang-format on */

void main() {
	// old code, ARM driver bug on Mali-GXXx GPUs and Vulkan API 1.3.xxx
	// https://github.com/godotengine/godot/pull/92817#issuecomment-2168625982
	//vec2 base_arr[3] = vec2[](vec2(-1.0, -1.0), vec2(-1.0, 3.0), vec2(3.0, -1.0));
	//gl_Position = vec4(base_arr[gl_VertexIndex], 0.0, 1.0);
	//uv_interp = clamp(gl_Position.xy, vec2(0.0, 0.0), vec2(1.0, 1.0)) * 2.0; // saturate(x) * 2.0

	vec2 vertex_base;
	if (gl_VertexIndex == 0) {
		vertex_base = vec2(-1.0, -1.0);
	} else if (gl_VertexIndex == 1) {
		vertex_base = vec2(-1.0, 3.0);
	} else {
		vertex_base = vec2(3.0, -1.0);
	}
	gl_Position = vec4(vertex_base, 0.0, 1.0);
	uv_interp = clamp(vertex_base, vec2(0.0, 0.0), vec2(1.0, 1.0)) * 2.0; // saturate(x) * 2.0
}

/* clang-format off */
#[fragment]

#version 450

layout(push_constant, std430) uniform Blur {
    vec2 source_pixel_size; // 08 - 8
    vec2 pad1; // 08 - 16

    // Glow.
    float glow_strength; // 04 - 20
    float glow_bloom; // 04 - 24
    float glow_hdr_threshold; // 04 - 28
    float glow_hdr_scale; // 04 - 32

    float glow_exposure; // 04 - 36
    float glow_luminance_cap; // 04 - 40
    float luminance_multiplier; // 04 - 44
    float pad2; // 04 - 48
}
blur;

layout(location = 0) in vec2 uv_interp;
/* clang-format on */

layout(set = 0, binding = 0) uniform sampler2D source_color;

layout(location = 0) out vec4 frag_color;

// https://www.shadertoy.com/view/mdsyDf
vec4 BloomDownKernel4(sampler2D Tex, vec2 uv0) {
	vec2 RcpSrcTexRes = blur.source_pixel_size;

	vec2 tc = (uv0 * 2.0 + 1.0) * RcpSrcTexRes;

	float la = 1.0 / 4.0;

	vec2 o = (0.5 + la) * RcpSrcTexRes;

	vec4 c = vec4(0.0);
	c += textureLod(Tex, tc + vec2(-1.0, -1.0) * o, 0.0) * 0.25;
	c += textureLod(Tex, tc + vec2(1.0, -1.0) * o, 0.0) * 0.25;
	c += textureLod(Tex, tc + vec2(-1.0, 1.0) * o, 0.0) * 0.25;
	c += textureLod(Tex, tc + vec2(1.0, 1.0) * o, 0.0) * 0.25;

	return c;
}

layout(constant_id = 0) const bool first_pass = false;

void main() {
	// We do not apply our color scale for our mobile renderer here, we'll leave our colors at half brightness and apply scale in the tonemap raster.

	if (first_pass) {
		// First step, go straight to quarter resolution.
		// Don't apply blur, but include thresholding.

		vec2 block_pos = floor(gl_FragCoord.xy) * 4.0;
		vec2 end = max(1.0 / blur.source_pixel_size - vec2(4.0), vec2(0.0));
		block_pos = clamp(block_pos, vec2(0.0), end);

		// We skipped a level, so gather 16 closest samples now.
		vec4 color = textureLod(source_color, (block_pos + vec2(0.5, 0.5)) * blur.source_pixel_size, 0.0);
		color += textureLod(source_color, (block_pos + vec2(0.5, 2.5)) * blur.source_pixel_size, 0.0);
		color += textureLod(source_color, (block_pos + vec2(2.5, 0.5)) * blur.source_pixel_size, 0.0);
		color += textureLod(source_color, (block_pos + vec2(2.5, 2.5)) * blur.source_pixel_size, 0.0);
		frag_color = color * 0.25;

		// Apply strength a second time since it usually gets added at each level.
		frag_color *= blur.glow_strength;
		frag_color *= blur.glow_strength;

		// In the first pass bring back to correct color range else we're applying the wrong threshold
		// in subsequent passes we can use it as is as we'd just be undoing it right after.
		frag_color *= blur.luminance_multiplier;
		frag_color *= blur.glow_exposure;

		float luminance = max(frag_color.r, max(frag_color.g, frag_color.b));
		float feedback = max(smoothstep(blur.glow_hdr_threshold, blur.glow_hdr_threshold + blur.glow_hdr_scale, luminance), blur.glow_bloom);

		frag_color = min(frag_color * feedback, vec4(blur.glow_luminance_cap)) / blur.luminance_multiplier;
	} else {
		// Regular downsample, apply a simple blur.
		frag_color = BloomDownKernel4(source_color, floor(gl_FragCoord.xy));
		frag_color *= blur.glow_strength;
	}
}
