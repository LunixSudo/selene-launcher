let context: AudioContext | null = null;
let noise: AudioBuffer | null = null;

/**
 * A short soft tap, like the iOS keyboard: a burst of filtered noise for the click and a low
 * tone under it for body. Made on the fly, so the launcher ships no sound files.
 */
export function playClick() {
  context ??= new AudioContext();
  noise ??= whiteNoise(context, 0.03);
  const now = context.currentTime;

  const tap = context.createBufferSource();
  tap.buffer = noise;
  const band = context.createBiquadFilter();
  band.type = "bandpass";
  band.frequency.value = 3200;
  band.Q.value = 0.9;
  const tapVolume = context.createGain();
  tapVolume.gain.setValueAtTime(0.32, now);
  tapVolume.gain.exponentialRampToValueAtTime(0.0001, now + 0.022);
  tap.connect(band).connect(tapVolume).connect(context.destination);

  const body = context.createOscillator();
  body.type = "sine";
  body.frequency.setValueAtTime(260, now);
  body.frequency.exponentialRampToValueAtTime(140, now + 0.03);
  const bodyVolume = context.createGain();
  bodyVolume.gain.setValueAtTime(0.12, now);
  bodyVolume.gain.exponentialRampToValueAtTime(0.0001, now + 0.035);
  body.connect(bodyVolume).connect(context.destination);

  tap.start(now);
  body.start(now);
  body.stop(now + 0.04);
}

function whiteNoise(context: AudioContext, seconds: number): AudioBuffer {
  const length = Math.ceil(context.sampleRate * seconds);
  const buffer = context.createBuffer(1, length, context.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < length; i++) {
    data[i] = Math.random() * 2 - 1;
  }
  return buffer;
}
