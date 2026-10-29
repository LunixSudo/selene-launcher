let context: AudioContext | null = null;

/** A short soft tick, made on the fly so the launcher ships no sound files. */
export function playClick() {
  context ??= new AudioContext();
  const now = context.currentTime;
  const tone = context.createOscillator();
  const volume = context.createGain();
  tone.type = "triangle";
  tone.frequency.setValueAtTime(1150, now);
  tone.frequency.exponentialRampToValueAtTime(620, now + 0.05);
  volume.gain.setValueAtTime(0.0001, now);
  volume.gain.exponentialRampToValueAtTime(0.07, now + 0.005);
  volume.gain.exponentialRampToValueAtTime(0.0001, now + 0.07);
  tone.connect(volume).connect(context.destination);
  tone.start(now);
  tone.stop(now + 0.08);
}
