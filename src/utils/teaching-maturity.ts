export type TeachingMaturity = 'untested' | 'learning' | 'proven' | 'canon';

export const PROVEN_MIN_USES = 10;
export const PROVEN_MIN_AVG_RATING = 4;

export interface TeachingEvidence {
  usage_count: number;
  rating_sum: number;
  rating_count: number;
  promoted_at?: number | null;
  enabled?: boolean;
}

export function averageRating(evidence: Pick<TeachingEvidence, 'rating_sum' | 'rating_count'>): number {
  if (evidence.rating_count === 0) return 0;
  return evidence.rating_sum / evidence.rating_count;
}

export function deriveTeachingMaturity(evidence: TeachingEvidence): TeachingMaturity {
  if (evidence.promoted_at !== null && evidence.promoted_at !== undefined) return 'canon';
  if (evidence.enabled === false || evidence.usage_count === 0) return 'untested';
  if (
    evidence.usage_count >= PROVEN_MIN_USES &&
    averageRating(evidence) >= PROVEN_MIN_AVG_RATING
  ) {
    return 'proven';
  }
  return 'learning';
}

export function teachingMaturityLabel(maturity: TeachingMaturity): string {
  switch (maturity) {
    case 'untested':
      return 'Untested';
    case 'learning':
      return 'Learning';
    case 'proven':
      return 'Proven';
    case 'canon':
      return 'Canon';
  }
}

export function teachingMaturityColor(maturity: TeachingMaturity): string {
  switch (maturity) {
    case 'untested':
      return 'var(--ts-text-muted)';
    case 'learning':
      return 'var(--ts-info)';
    case 'proven':
      return 'var(--ts-success)';
    case 'canon':
      return 'var(--ts-accent-violet)';
  }
}

export function formatRelativeTime(epochMs: number, nowMs = Date.now()): string {
  const seconds = Math.max(0, Math.round((nowMs - epochMs) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}