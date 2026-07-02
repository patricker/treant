// Tiny synthesized sound effects via the Web Audio API — no asset files, no deps.
// All sounds are short oscillator + gain envelopes. Muting is persisted in
// localStorage; the AudioContext is created lazily and resumed on first gesture
// to satisfy browser autoplay policies.

let ctx: AudioContext | null = null;

function audioCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  if (!ctx) {
    const AC = window.AudioContext || (window as any).webkitAudioContext;
    if (!AC) return null;
    ctx = new AC();
  }
  return ctx;
}

export function isMuted(): boolean {
  if (typeof window === 'undefined') return true;
  return window.localStorage.getItem('arcade.muted') === '1';
}

export function setMuted(m: boolean): void {
  if (typeof window !== 'undefined') {
    window.localStorage.setItem('arcade.muted', m ? '1' : '0');
  }
}

/** Resume the AudioContext on a user gesture (call from the mute toggle / first tap). */
export function unlockAudio(): void {
  const c = audioCtx();
  if (c && c.state === 'suspended') void c.resume();
}

function blip(
  freqs: number[],
  dur: number,
  type: OscillatorType = 'square',
  gain = 0.05,
): void {
  if (isMuted()) return;
  const c = audioCtx();
  if (!c) return;
  if (c.state === 'suspended') void c.resume();
  const now = c.currentTime;
  freqs.forEach((f, i) => {
    const osc = c.createOscillator();
    const g = c.createGain();
    osc.type = type;
    osc.frequency.value = f;
    const t0 = now + i * dur * 0.8;
    g.gain.setValueAtTime(0, t0);
    g.gain.linearRampToValueAtTime(gain, t0 + 0.01);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    osc.connect(g);
    g.connect(c.destination);
    osc.start(t0);
    osc.stop(t0 + dur);
  });
}

// Haptic feedback to pair with the sound effects. Uses the standard Vibration
// API (navigator.vibrate), which Android's WebView and the packaged Capacitor
// app both honour on a user gesture; iOS/Safari ignore it silently, which is
// acceptable. Gated on the same mute switch as sound so a muted game is fully
// silent and still. The optional-chaining call is a no-op where unsupported.
function buzz(pattern: number | number[]): void {
  if (isMuted()) return;
  if (typeof navigator !== 'undefined') navigator.vibrate?.(pattern);
}

export const sound = {
  move: () => {
    blip([330], 0.07, 'square', 0.04);
    buzz(15);
  },
  drop: () => {
    blip([200, 120], 0.13, 'sine', 0.07);
    buzz(15);
  },
  merge: () => {
    blip([520, 700], 0.1, 'triangle', 0.05);
    buzz(15);
  },
  win: () => {
    blip([523, 659, 784, 1047], 0.13, 'square', 0.05);
    buzz([15, 40, 15, 40, 15]);
  },
  lose: () => blip([330, 247, 165], 0.18, 'sawtooth', 0.04),
  draw: () => blip([392, 392], 0.12, 'sine', 0.045),
};
