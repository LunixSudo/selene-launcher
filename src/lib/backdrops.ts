export type ShaderBackdrop = "halo" | "rays" | "aurora" | "liquid" | "bokeh" | "waves";

export const SHADER_BACKDROPS: ShaderBackdrop[] = [
  "halo",
  "rays",
  "aurora",
  "liquid",
  "bokeh",
  "waves",
];

/** The canvas renders at this share of the window size; soft light needs no more. */
const RENDER_SCALE = 0.5;

const VERTEX = `
attribute vec2 position;
void main() {
  gl_Position = vec4(position, 0.0, 1.0);
}
`;

const COMMON = `
// Old GPUs run mediump at half precision, which breaks the noise into blocks.
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_accent;
uniform float u_light;
uniform vec4 u_background;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
    u.y
  );
}

float fbm(vec2 p) {
  float value = 0.0;
  float amplitude = 0.5;
  for (int i = 0; i < 4; i++) {
    value += amplitude * noise(p);
    p *= 2.03;
    amplitude *= 0.5;
  }
  return value;
}

vec3 spectrum(float t) {
  return 0.5 + 0.5 * cos(6.28318 * (t + vec3(0.0, 0.33, 0.67)));
}

// Light themes get the same colours, a little softer, laid over the page instead of glowing.
// The output is premultiplied, so light adds up the way a glow does. Over a solid page the
// page colour is blended in here: an opaque canvas leaves old GPUs nothing to composite wrong.
void finish(vec3 color) {
  color = min(color, 1.0) * mix(1.0, 0.7, u_light);
  float alpha = max(color.r, max(color.g, color.b));
  color += u_background.rgb * (1.0 - alpha) * u_background.a;
  alpha = mix(alpha, 1.0, u_background.a);
  // A touch of noise hides the steps an 8-bit screen makes of soft gradients.
  float dither = (hash(gl_FragCoord.xy + fract(u_time)) - 0.5) / 255.0;
  gl_FragColor = clamp(vec4(color + dither, alpha + dither), 0.0, 1.0);
}
`;

/** Three living rings that wobble, drift and run through the colours, like the iOS glow. */
const HALO = `
void main() {
  vec2 p = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;
  p.x -= 0.12;
  float t = u_time;
  vec3 color = vec3(0.0);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    vec2 q = p - 0.07 * vec2(sin(t * 0.21 + fi * 2.1), cos(t * 0.17 + fi * 1.7));
    float angle = atan(q.y, q.x);
    float wobble = 0.035 * sin(angle * 3.0 + t * 0.6 + fi * 2.0)
      + 0.022 * sin(angle * 5.0 - t * 0.45 + fi * 4.0)
      + 0.04 * (fbm(normalize(q) * 1.5 + vec2(fi * 10.0, t * 0.12)) - 0.5);
    float radius = 0.27 + 0.04 * fi + wobble;
    float d = abs(length(q) - radius);
    float core = exp(-d * d / 0.0009);
    float glow = exp(-d * 7.0) * 0.45;
    float hue = t * 0.035 + fi * 0.27 + angle / 6.28318;
    vec3 tint = mix(spectrum(hue), u_accent, 0.25);
    color += tint * (core * 0.8 + glow) * 0.62;
  }
  finish(color);
}
`;

/** Soft beams of light falling from the top corner, with dust glinting in them. */
const RAYS = `
void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  float aspect = u_resolution.x / u_resolution.y;
  vec2 d = uv - vec2(0.08, 1.12);
  d.x *= aspect;
  float t = u_time;
  float angle = atan(d.x, -d.y);
  float dist = length(d);

  float beams = pow(fbm(vec2(angle * 6.0, t * 0.07)), 3.0) * 2.4
    + pow(noise(vec2(angle * 22.0 + 3.0, t * 0.2)), 6.0) * 1.4;
  float cone = smoothstep(-0.2, 0.25, angle) * smoothstep(1.6, 0.95, angle);
  float fall = exp(-dist * 0.85);
  float light = beams * cone * fall;

  // Dust drifting through the light: round specks that fade in and out.
  vec2 g = uv * vec2(aspect, 1.0) * 26.0 + vec2(t * 0.25, -t * 0.45);
  vec2 cell = floor(g);
  vec2 jitter = vec2(hash(cell), hash(cell + 3.1)) - 0.5;
  float speck = smoothstep(0.16, 0.0, length(fract(g) - 0.5 - jitter * 0.6));
  float twinkle = 0.5 + 0.5 * sin(t * 1.4 + hash(cell + 5.0) * 40.0);
  float dust = step(0.9, hash(cell + 11.0)) * speck * twinkle;

  vec3 prism = spectrum(angle * 0.9 + t * 0.03);
  vec3 tint = mix(vec3(1.0, 0.95, 0.88), mix(prism, u_accent, 0.35), 0.55);
  vec3 color = tint * light * 1.35
    + vec3(1.0, 0.97, 0.9) * dust * cone * fall * 0.9
    + u_accent * exp(-dist * 2.4) * 0.4;
  finish(color);
}
`;

/** Curtains of northern lights that ripple and slowly change colour. */
const AURORA = `
void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  float t = u_time;
  vec3 color = vec3(0.0);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float x = uv.x * (1.3 + fi * 0.4) + fi * 3.1;
    float base = 0.5 + fi * 0.1 + 0.22 * (fbm(vec2(x * 0.8 + t * 0.05, fi * 5.0 + t * 0.04)) - 0.5);
    float dy = uv.y - base;
    float curtain = dy > 0.0 ? exp(-dy * (2.6 + fi * 1.5)) : exp(dy * 30.0);
    float streaks = pow(0.35 + 0.65 * noise(vec2(x * 16.0 + t * 0.3, fi * 3.0 + t * 0.05)), 1.6);
    float shift = 0.5 + 0.5 * sin(fi * 1.3 + t * 0.09 + uv.x * 2.5);
    vec3 tint = mix(vec3(0.15, 1.0, 0.55), vec3(0.1, 0.8, 0.95), shift);
    // The upper edge of each curtain turns violet, as the real ones do.
    tint = mix(tint, mix(vec3(0.62, 0.35, 1.0), u_accent, 0.4), smoothstep(0.04, 0.3, dy));
    color += tint * curtain * streaks * 0.85;
  }
  color += u_accent * smoothstep(0.4, 0.0, uv.y) * 0.1;
  finish(color);
}
`;

/** A slow flowing mesh of colour, like liquid glass. */
const LIQUID = `
void main() {
  float aspect = u_resolution.x / u_resolution.y;
  vec2 uv = gl_FragCoord.xy / u_resolution.y;
  float t = u_time * 0.12;
  vec3 colors[4];
  colors[0] = u_accent;
  colors[1] = mix(vec3(1.0, 0.3, 0.65), spectrum(t * 0.1 + 0.9), 0.3);
  colors[2] = vec3(0.1, 0.8, 0.95);
  colors[3] = mix(vec3(1.0, 0.6, 0.25), spectrum(t * 0.1 + 0.3), 0.3);
  vec3 color = vec3(0.0);
  float total = 0.0;
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    vec2 center = vec2(
      (0.5 + 0.42 * sin(t * (0.7 + fi * 0.13) + fi * 1.9)) * aspect,
      0.5 + 0.42 * cos(t * (0.6 + fi * 0.17) + fi * 2.7)
    );
    vec2 warped = uv + 0.08 * vec2(sin(uv.y * 3.0 + t), cos(uv.x * 3.0 - t));
    float weight = 1.0 / (0.002 + pow(length(warped - center), 4.0));
    color += colors[i] * weight;
    total += weight;
  }
  finish(color / total * 0.5);
}
`;

/** Out-of-focus lights drifting up, as in a night photo. */
const BOKEH = `
void main() {
  float aspect = u_resolution.x / u_resolution.y;
  vec2 uv = gl_FragCoord.xy / u_resolution.y;
  float t = u_time;
  vec3 color = vec3(0.0);
  for (int i = 0; i < 26; i++) {
    float fi = float(i);
    float h1 = hash(vec2(fi, 1.7));
    float h2 = hash(vec2(fi, 4.1));
    float h3 = hash(vec2(fi, 8.3));
    float size = 0.03 + 0.08 * h3;
    vec2 center = vec2(
      fract(h1 + 0.02 * sin(t * 0.2 + fi)) * aspect,
      fract(h2 * 7.0 + t * (0.008 + 0.02 * h2)) * 1.4 - 0.2
    );
    float d = length(uv - center);
    float disc = smoothstep(size, size * 0.7, d) * (0.6 + 0.4 * smoothstep(size * 0.3, size, d));
    float twinkle = 0.75 + 0.25 * sin(t * (0.5 + h2) + fi);
    vec3 tint = mix(spectrum(h3 + t * 0.01), u_accent, 0.35);
    color += tint * disc * twinkle * (0.18 + 0.2 * h1);
  }
  finish(color);
}
`;

/** Silky lines of light that ripple across the window. */
const WAVES = `
void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  float t = u_time * 0.25;
  vec3 color = vec3(0.0);
  for (int i = 0; i < 5; i++) {
    float fi = float(i);
    float y = 0.42 + fi * 0.045
      + 0.09 * sin(uv.x * (2.2 + fi * 0.35) + t * (1.0 + fi * 0.2) + fi * 1.3)
      + 0.05 * sin(uv.x * 5.1 - t * 0.7 + fi * 2.1);
    float d = abs(uv.y - y);
    float line = exp(-d * d / 0.00006) * 0.7 + exp(-d * 18.0) * 0.22;
    vec3 tint = mix(u_accent, spectrum(fi * 0.12 + uv.x * 0.3 + t * 0.05), 0.55);
    color += tint * line;
  }
  finish(color);
}
`;

const FRAGMENTS: Record<ShaderBackdrop, string> = {
  halo: HALO,
  rays: RAYS,
  aurora: AURORA,
  liquid: LIQUID,
  bokeh: BOKEH,
  waves: WAVES,
};

function compile(gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) {
    return null;
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
}

function accentColor(): [number, number, number] {
  const value = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim();
  const hex = /^#([0-9a-f]{6})$/i.exec(value)?.[1] ?? "6b5bff";
  return [0, 2, 4].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255) as [
    number,
    number,
    number,
  ];
}

/** The solid colour behind the canvas, or all zeros when the window itself shows through. */
function pageColor(canvas: HTMLCanvasElement): [number, number, number, number] {
  if (!canvas.isConnected) {
    return [0, 0, 0, 0];
  }
  const channels = getComputedStyle(document.body)
    .backgroundColor.match(/[\d.]+/g)
    ?.map(Number);
  if (!channels || channels.length < 3 || (channels[3] ?? 1) < 1) {
    return [0, 0, 0, 0];
  }
  return [channels[0] / 255, channels[1] / 255, channels[2] / 255, 1];
}

function isLight(): boolean {
  const theme = document.documentElement.dataset.theme;
  return (
    theme === "light" || (theme !== "dark" && matchMedia("(prefers-color-scheme: light)").matches)
  );
}

interface Renderer {
  /** Draws a frame at `seconds`; `restyle` reads the accent colour and theme again. */
  draw(seconds: number, restyle: boolean): void;
  dispose(): void;
}

function createRenderer(canvas: HTMLCanvasElement, kind: ShaderBackdrop): Renderer | null {
  const gl = canvas.getContext("webgl", { antialias: false, preserveDrawingBuffer: false });
  const vertex = gl && compile(gl, gl.VERTEX_SHADER, VERTEX);
  const fragment = gl && compile(gl, gl.FRAGMENT_SHADER, COMMON + FRAGMENTS[kind]);
  const program = gl?.createProgram();
  if (!gl || !vertex || !fragment || !program) {
    return null;
  }
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    return null;
  }
  gl.useProgram(program);

  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  const position = gl.getAttribLocation(program, "position");
  gl.enableVertexAttribArray(position);
  gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

  const resolution = gl.getUniformLocation(program, "u_resolution");
  const time = gl.getUniformLocation(program, "u_time");
  const accent = gl.getUniformLocation(program, "u_accent");
  const light = gl.getUniformLocation(program, "u_light");
  const background = gl.getUniformLocation(program, "u_background");

  return {
    draw(seconds, restyle) {
      if (restyle) {
        gl.uniform3fv(accent, accentColor());
        gl.uniform1f(light, isLight() ? 1 : 0);
        gl.uniform4fv(background, pageColor(canvas));
      }
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.uniform2f(resolution, canvas.width, canvas.height);
      gl.uniform1f(time, seconds);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    },
    dispose() {
      gl.getExtension("WEBGL_lose_context")?.loseContext();
    },
  };
}

/**
 * Draws an animated background into `canvas` until the returned function is called. Returns
 * null when the PC has no WebGL.
 */
export function startShader(canvas: HTMLCanvasElement, kind: ShaderBackdrop): (() => void) | null {
  const renderer = createRenderer(canvas, kind);
  if (!renderer) {
    return null;
  }
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  // Each start picks up somewhere else in the animation, so it never opens on the same frame.
  const offset = Math.random() * 300;
  let frame = 0;
  let drawn = 0;

  const draw = (now: number) => {
    // Reading styles every frame is costly; once a second still follows a theme switch.
    renderer.draw(offset + (still ? 0 : now / 1000), still || drawn++ % 60 === 0);
    if (!still) {
      frame = requestAnimationFrame(draw);
    }
  };
  const resize = () => {
    canvas.width = Math.max(1, Math.round(canvas.clientWidth * RENDER_SCALE));
    canvas.height = Math.max(1, Math.round(canvas.clientHeight * RENDER_SCALE));
    // Resizing clears the canvas, and a still picture is not redrawn on its own.
    if (still) {
      frame = requestAnimationFrame(draw);
    }
  };

  resize();
  const observer = new ResizeObserver(resize);
  observer.observe(canvas);
  if (!still) {
    frame = requestAnimationFrame(draw);
  }

  return () => {
    cancelAnimationFrame(frame);
    observer.disconnect();
    renderer.dispose();
  };
}

const previews = new Map<string, string>();

/** One frame of a background as a picture for the settings, or null without WebGL. */
export function previewImage(kind: ShaderBackdrop): string | null {
  const key = `${kind} ${accentColor().join()} ${isLight()}`;
  const cached = previews.get(key);
  if (cached) {
    return cached;
  }
  const canvas = document.createElement("canvas");
  canvas.width = 208;
  canvas.height = 128;
  const renderer = createRenderer(canvas, kind);
  if (!renderer) {
    return null;
  }
  renderer.draw(40, true);
  const image = canvas.toDataURL();
  renderer.dispose();
  previews.set(key, image);
  return image;
}
