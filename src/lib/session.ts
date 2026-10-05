import type { SoundKind } from './sound'

// Session states, see docs/fonctionnel/DF-0001-etats-du-notch.md
export type SessionState =
  | 'idle'
  | 'working'
  | 'needs_permission'
  | 'waiting_input'
  | 'done'
  | 'error'

// Mirrors `Session` in src-tauri/src/sessions.rs
export interface Session {
  id: string
  cwd: string | null
  state: SessionState
  tool: string | null
  startedAtMs: number
}

// Screen edge the notch is attached to, mirrors `Edge` in src-tauri/src/placement.rs
export type Edge = 'top' | 'bottom' | 'left' | 'right'

// Mirrors `Status` in src-tauri/src/lib.rs
export interface Status {
  serverError: string | null
  hooksInstalled: boolean
  soundEnabled: boolean
  edge: Edge
  // Move mode: the notch can be dragged along its edge (DF-0006)
  movable: boolean
  // Center of the compact notch along the edge, from the start of the window,
  // in logical pixels; null = the middle of the window (DF-0006)
  anchor: number | null
}

export const stateLabels: Record<SessionState, string> = {
  idle: 'En attente',
  working: 'Au travail',
  needs_permission: 'Permission requise',
  waiting_input: 'Attend ta réponse',
  done: 'Terminé',
  error: 'Erreur',
}

// Most urgent first: decides the notch colour when several sessions run.
const priority: SessionState[] = [
  'needs_permission',
  'error',
  'waiting_input',
  'done',
  'working',
  'idle',
]

export function mostUrgent(sessions: Session[]): Session | undefined {
  return [...sessions].sort(
    (a, b) => priority.indexOf(a.state) - priority.indexOf(b.state),
  )[0]
}

export function stateColor(state: SessionState): string {
  return `var(--state-${state.replaceAll('_', '-')})`
}

export function projectName(session: Session): string {
  return session.cwd?.split(/[\\/]/).filter(Boolean).pop() ?? 'session'
}

export function describe(session: Session): string {
  const tool = session.tool ? ` (${session.tool})` : ''
  return `${projectName(session)} : ${stateLabels[session.state]}${tool}`
}

// States that deserve attention: they expand the notch and play a sound (DF-0003).
export function soundFor(state: SessionState): SoundKind | undefined {
  switch (state) {
    case 'needs_permission':
    case 'waiting_input':
      return 'attention'
    case 'done':
      return 'done'
    case 'error':
      return 'error'
    default:
      return undefined
  }
}

// Sessions that just entered an attention state, compared to the previous list.
export function newAlerts(previous: Session[], next: Session[]): Session[] {
  const before = new Map(previous.map((s) => [s.id, s.state]))
  return next.filter((s) => soundFor(s.state) && before.get(s.id) !== s.state)
}
