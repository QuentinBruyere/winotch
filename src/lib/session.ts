// Session states, see docs/fonctionnel/DF-0001-etats-du-notch.md
export type SessionState =
  | 'idle'
  | 'working'
  | 'needs_permission'
  | 'waiting_input'
  | 'done'
  | 'error'

export const stateLabels: Record<SessionState, string> = {
  idle: 'En attente',
  working: 'Au travail',
  needs_permission: 'Permission requise',
  waiting_input: 'Attend ta réponse',
  done: 'Terminé',
  error: 'Erreur',
}

export function stateColor(state: SessionState): string {
  return `var(--state-${state.replaceAll('_', '-')})`
}
