// Notification sounds, synthesized: no third-party audio files (DF-0003).

export type SoundKind = 'attention' | 'done' | 'error'

let context: AudioContext | undefined

function note(ctx: AudioContext, frequency: number, start: number, duration: number) {
  const osc = ctx.createOscillator()
  const gain = ctx.createGain()
  osc.type = 'sine'
  osc.frequency.value = frequency
  gain.gain.setValueAtTime(0.0001, start)
  gain.gain.exponentialRampToValueAtTime(0.18, start + 0.015)
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration)
  osc.connect(gain).connect(ctx.destination)
  osc.start(start)
  osc.stop(start + duration + 0.05)
}

export function playSound(kind: SoundKind) {
  context ??= new AudioContext()
  const ctx = context
  void ctx.resume()
  const t = ctx.currentTime + 0.02
  switch (kind) {
    case 'attention': // two rising notes: something to do
      note(ctx, 659.25, t, 0.18)
      note(ctx, 987.77, t + 0.17, 0.3)
      break
    case 'done': // soft major arpeggio
      note(ctx, 523.25, t, 0.5)
      note(ctx, 659.25, t + 0.07, 0.5)
      note(ctx, 783.99, t + 0.14, 0.6)
      break
    case 'error':
      note(ctx, 196, t, 0.45)
      break
  }
}
