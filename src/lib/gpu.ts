/** Graphics that struggle with live backgrounds and blur: old or built-in chips, or software. */
const WEAK = [
  /swiftshader|basic render|llvmpipe|software/i,
  /\bGMA\b|Mobile Intel/i,
  /(?:^|[^U])HD Graphics\b/i,
  /GeForce (?:[2-9]\d{3}|[1-5]\d{2})M?\b/i,
  /GeForce GT \d{3}\b/i,
  /Radeon (?:HD [2-7]\d{3}|R[57] )/i,
];

export interface Gpu {
  name: string;
  weak: boolean;
}

/** The graphics card as the webview reports it, or null when it keeps that to itself. */
export function detectGpu(): Gpu | null {
  try {
    const gl = document.createElement("canvas").getContext("webgl");
    const info = gl?.getExtension("WEBGL_debug_renderer_info");
    const renderer: unknown = gl && info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : null;
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    if (typeof renderer !== "string" || !renderer) {
      return null;
    }
    return {
      name: tidy(renderer) || renderer,
      weak: WEAK.some((pattern) => pattern.test(renderer)),
    };
  } catch {
    return null;
  }
}

/**
 * "ANGLE (NVIDIA, NVIDIA GeForce 310M (0x00000A70) Direct3D11 vs_5_0 ps_5_0, D3D11)" → "GeForce 310M",
 * "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)"
 * → "SwiftShader Device (Subzero)".
 */
function tidy(renderer: string): string {
  const inner = renderer.match(/^ANGLE \([^,]*,\s*([^,]+)/)?.[1] ?? renderer;
  const device = inner.match(/^(?:Vulkan|OpenGL)[\d. ]*\((.+)\)$/)?.[1] ?? inner;
  return device
    .replace(/\(0x[0-9a-f]+\)/gi, "")
    .replace(/(?:^|\s)(?:Direct3D|D3D|OpenGL|Vulkan).*$/i, "")
    .replace(/\((?:R|TM)\)/gi, "")
    .replace(/^(?:NVIDIA|AMD|ATI|Intel)\s+/i, "")
    .replace(/\s+/g, " ")
    .trim();
}
