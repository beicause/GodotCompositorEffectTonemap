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
    vec2 dest_pixel_size; // 08 - 08
    vec2 source_pixel_size; // 08 - 16

    float glow_level; // 04 - 20
    // Glow.
    float glow_strength; // 04 - 24
    vec2 pad; // 08 - 32
}
blur;

layout(location = 0) in vec2 uv_interp;
/* clang-format on */

layout(set = 0, binding = 0) uniform sampler2D source_color;

// When upsampling this is original downsampled texture, not the blended upsampled texture.
layout(set = 1, binding = 0) uniform sampler2D blend_color;
layout(constant_id = 0) const bool use_blend_color = false;

layout(location = 0) out vec4 frag_color;

// https://www.shadertoy.com/view/mdsyDf
vec4 BloomUpKernel4(sampler2D Tex, vec2 uv0) {
	vec2 RcpSrcTexRes = blur.source_pixel_size;

	vec2 uv = uv0 * 0.5 + 0.5;

	vec2 uvI = floor(uv);
	vec2 uvF = uv - uvI;

	vec2 tc = uvI * RcpSrcTexRes.xy;

	// optimal stop-band
	float lw = 0.357386;
	float la = 25.0 / 32.0; // 0.78125  ~ 0.779627;
	float lb = 3.0 / 64.0; // 0.046875 ~ 0.0493871;

	vec2 l = vec2(-1.5 + la, 0.5 + lb);

	vec2 lx = uvF.x == 0.0 ? l.xy : -l.yx;
	vec2 ly = uvF.y == 0.0 ? l.xy : -l.yx;

	lx *= RcpSrcTexRes.xx;
	ly *= RcpSrcTexRes.yy;

	vec4 c00 = textureLod(Tex, tc + vec2(lx.x, ly.x), 0.0);
	vec4 c10 = textureLod(Tex, tc + vec2(lx.y, ly.x), 0.0);
	vec4 c01 = textureLod(Tex, tc + vec2(lx.x, ly.y), 0.0);
	vec4 c11 = textureLod(Tex, tc + vec2(lx.y, ly.y), 0.0);

	vec2 w = abs(uvF * 2.0 - lw);

	vec4 cx0 = c00 * (1.0 - w.x) + (c10 * w.x);
	vec4 cx1 = c01 * (1.0 - w.x) + (c11 * w.x);

	vec4 cxy = cx0 * (1.0 - w.y) + (cx1 * w.y);

	return cxy;
}

// very good approximation of BloomUpKernel8; good radial symmetry
// but more aliasing than BloomUpKernel4
// vec4 BloomUpKernel4B(sampler2D Tex, vec2 uv0)
// {
//     vec2 RcpSrcTexRes = blur.source_pixel_size;

//     vec2 uv = uv0 * 0.5 + 0.5;

//     vec2 uvI = floor(uv);
//     vec2 uvF = uv - uvI;

//     vec2 tc = uvI * RcpSrcTexRes.xy;

//     vec2 l00 = vec2(11.0 / 32.0, 17.0 / 32.0); // 0.34375  ~ 0.347209
//     vec2 l10 = vec2(7.0 / 64.0, 11.0 / 32.0); // 0.109375 ~ 0.109840
//     vec2 l01 = vec2(11.0 / 32.0, 7.0 / 64.0); // 0.34375  ~ 0.334045
//     vec2 l11 = vec2(17.0 / 32.0, 11.0 / 32.0); // 0.53125  ~ 0.526425

//     vec4 w = vec4(0.288971, 0.211029, 0.211029, 0.288971);

//     bool flipX = uvF.x != 0.0;
//     bool flipY = uvF.y != 0.0;

//     if (flipX)
//     {
//         vec2 tmp = l11;
//         l11 = l10;
//         l10 = tmp;

//         l00.x = 1.0 - l00.x;
//         l10.x = 1.0 - l10.x;
//         l01.x = 1.0 - l01.x;
//         l11.x = 1.0 - l11.x;

//         w = vec4(w.x, w.w, w.z, w.y);
//     }

//     if (flipY)
//     {
//         vec2 tmp = l00;
//         l00 = l01;
//         l01 = tmp;

//         l00.y = 1.0 - l00.y;
//         l10.y = 1.0 - l10.y;
//         l01.y = 1.0 - l01.y;
//         l11.y = 1.0 - l11.y;

//         w = vec4(w.z, w.y, w.x, w.w);
//     }

//     vec4 col = vec4(0.0);

//     col += textureLod(Tex, tc + (vec2(-0.5, -1.5) + l00) * RcpSrcTexRes, 0.0) * w.x;
//     col += textureLod(Tex, tc + (vec2(0.5, -0.5) + l10) * RcpSrcTexRes, 0.0) * w.y;
//     col += textureLod(Tex, tc + (vec2(-0.5, 0.5) + l01) * RcpSrcTexRes, 0.0) * w.z;
//     col += textureLod(Tex, tc + (vec2(-1.5, -0.5) + l11) * RcpSrcTexRes, 0.0) * w.w;

//     return col;
// }

void main() {
	// We do not apply our color scale for our mobile renderer here, we'll leave our colors at half brightness and apply scale in the tonemap raster.

	frag_color = BloomUpKernel4(source_color, floor(gl_FragCoord.xy)) * blur.glow_strength; // "glow_strength" here is actually the glow level. It is always 1.0, except for the first upsample where we need to apply the level to two textures at once.
	if (use_blend_color) {
		vec2 uv = floor(gl_FragCoord.xy) + 0.5;
		frag_color += textureLod(blend_color, uv * blur.dest_pixel_size, 0.0) * blur.glow_level;
	}
}
