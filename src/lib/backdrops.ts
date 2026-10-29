export type ShaderBackdrop = "halo" | "rays" | "aurora";

/** The canvas renders at this share of the window size; soft light needs no more. */
const RENDER_SCALE = 0.5;

const VERTEX = `
attribute vec2 position;
void main() {
  gl_Position = vec4(position, 0.0, 1.0);
}
`;

const COMMON = `
precision mediump float;
uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_accent;
uniform float u_light;

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

// Light themes get the same colours, a little deeper, laid over the page instead of glowing.
void finish(vec3 color) {
  color = mix(color, color * 0.8, u_light);
  float alpha = clamp(max(color.r, max(color.g, color.b)), 0.0, 1.0);
  // Without premultiplied alpha the page blends this as colour * alpha, so dividing first
  // lets the light add up the way a glow does.
  gl_FragColor = vec4(min(color / max(alpha, 0.001), 1.0), alpha * mix(1.0, 0.75, u_light));
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
      + 0.04 * (fbm(vec2(angle * 1.5 + fi * 10.0, t * 0.12)) - 0.5);
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

const FRAGMENTS: Record<ShaderBackdrop, string> = { halo: HALO, rays: RAYS, aurora: AURORA };

function compile(gl: WebGLRenderingContext, type: number, source: string): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) {
    return null;
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  return gl.getShaderParameter(shader, gl.COMPILE_STATUS) ? shader : null;
}

function accentColor(element: HTMLElement): [number, number, number] {
  const value = getComputedStyle(element).getPropertyValue("--accent").trim();
  const hex = /^#([0-9a-f]{6})$/i.exec(value)?.[1] ?? "6b5bff";
  return [0, 2, 4].map((at) => Number.parseInt(hex.slice(at, at + 2), 16) / 255) as [
    number,
    number,
    number,
  ];
}

function isLight(): boolean {
  const theme = document.documentElement.dataset.theme;
  return (
    theme === "light" || (theme !== "dark" && matchMedia("(prefers-color-scheme: light)").matches)
  );
}

/**
 * Draws an animated background into `canvas` until the returned function is called. Returns
 * null when the PC has no WebGL.
 */
export function startShader(canvas: HTMLCanvasElement, kind: ShaderBackdrop): (() => void) | null {
  const gl = canvas.getContext("webgl", { premultipliedAlpha: false, antialias: false });
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

  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  // Each start picks up somewhere else in the animation, so it never opens on the same frame.
  const offset = Math.random() * 1000;
  let frame = 0;
  let drawn = 0;

  const resize = () => {
    canvas.width = Math.max(1, Math.round(canvas.clientWidth * RENDER_SCALE));
    canvas.height = Math.max(1, Math.round(canvas.clientHeight * RENDER_SCALE));
    gl.viewport(0, 0, canvas.width, canvas.height);
    if (still) {
      frame = requestAnimationFrame(draw);
    }
  };
  const draw = (now: number) => {
    // Reading styles every frame is costly; once a second still follows a theme switch.
    if (still || drawn++ % 60 === 0) {
      gl.uniform3fv(accent, accentColor(canvas));
      gl.uniform1f(light, isLight() ? 1 : 0);
    }
    gl.uniform2f(resolution, canvas.width, canvas.height);
    gl.uniform1f(time, offset + (still ? 0 : now / 1000));
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    if (!still) {
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
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  };
}
