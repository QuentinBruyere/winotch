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

// Mirrors `Status` in src-tauri/src/lib.rs
export interface Status {
  serverError: string | null
  hooksInstalled: boolean
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
