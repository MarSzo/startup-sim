// The world's "paper and ink" look (Don't Starve-ish), over the game world
// and under the HUD: ink lines where colours change sharply, warm muted
// colours, paper grain and a vignette.
shader_type canvas_item;

uniform sampler2D screen_tex : hint_screen_texture, filter_linear;
uniform float ink_strength = 0.85;
uniform float ink_threshold = 0.12;
uniform float line_width = 1.5;
uniform float desaturate = 0.22;
uniform float grain = 0.05;
uniform float vignette = 0.6;

float lum(vec2 uv) {
	return dot(texture(screen_tex, uv).rgb, vec3(0.299, 0.587, 0.114));
}

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

void fragment() {
	vec2 uv = SCREEN_UV;
	vec2 d = SCREEN_PIXEL_SIZE * line_width;
	vec3 c = texture(screen_tex, uv).rgb;
	// Sobel on luminance: sharp changes become ink.
	float tl = lum(uv + vec2(-d.x, -d.y));
	float t = lum(uv + vec2(0.0, -d.y));
	float tr = lum(uv + vec2(d.x, -d.y));
	float l = lum(uv + vec2(-d.x, 0.0));
	float r = lum(uv + vec2(d.x, 0.0));
	float bl = lum(uv + vec2(-d.x, d.y));
	float b = lum(uv + vec2(0.0, d.y));
	float br = lum(uv + vec2(d.x, d.y));
	float gx = -tl - 2.0 * l - bl + tr + 2.0 * r + br;
	float gy = -tl - 2.0 * t - tr + bl + 2.0 * b + br;
	float edge = smoothstep(ink_threshold, ink_threshold + 0.25, length(vec2(gx, gy)));
	// Muted, warm (sepia-ish) colours with a bit more contrast.
	float y = dot(c, vec3(0.299, 0.587, 0.114));
	c = mix(c, vec3(y), desaturate);
	c *= vec3(1.04, 0.97, 0.84);
	c = (c - 0.5) * 1.08 + 0.5;
	// Ink.
	c = mix(c, vec3(0.13, 0.10, 0.07), edge * ink_strength);
	// Paper grain (in 2x2 blocks so it reads as texture, not noise).
	float n = hash(floor(FRAGCOORD.xy / 2.0));
	c += (n - 0.5) * grain;
	// Vignette.
	float v = distance(uv, vec2(0.5));
	c *= 1.0 - smoothstep(0.35, 0.9, v) * vignette;
	COLOR = vec4(c, 1.0);
}
