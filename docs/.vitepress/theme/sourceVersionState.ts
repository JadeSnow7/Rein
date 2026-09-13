import type { InjectionKey, Ref } from 'vue'

export type Language = 'ts' | 'rust'
export const sourceVersionStateKey: InjectionKey<Ref<Language>> = Symbol('rein-source-version')

export type SourceTopic = 'reading-00' | 'chapter-01'

export type SourceEdition = {
  shared: string
  ts: string
  rust?: string
  commonHashes: string[]
  hashAliases?: Record<string, string>
}

export const sourceEditions: Record<SourceTopic, SourceEdition> = {
  'reading-00': {
    shared: '/readings/00.html',
    ts: '/readings/00-ts.html',
    rust: '/readings/00-rust.html',
    commonHashes: ['setup', 'types', 'json', 'errors', 'async', 'source', 'exercises', 'exercise-01', 'exercise-02', 'exercise-03', 'comparison'],
    hashAliases: { ownership: 'types', modules: 'source' }
  },
  'chapter-01': {
    shared: '/chapters/01.html',
    ts: '/chapters/01-ts.html',
    rust: '/chapters/01-rust.html',
    commonHashes: ['from-reading-0', 'first-run', 'live-call', 'request-response', 'implementation', 'failures', 'transport', 'verification', 'recording', 'exercises', 'exercise-01', 'exercise-02', 'exercise-03', 'exercise-04', 'comparison', 'next']
  }
}

export const resolveAnchorHash = (headingId: string | undefined, edition: SourceEdition, parentHeadingId?: string): string => {
  const target = edition.hashAliases?.[headingId ?? ''] ?? headingId
  if (target && edition.commonHashes.includes(target)) return `#${target}`
  const parentTarget = edition.hashAliases?.[parentHeadingId ?? ''] ?? parentHeadingId
  return parentTarget && edition.commonHashes.includes(parentTarget) ? `#${parentTarget}` : ''
}

export const sourceTopicForPath = (path: string): SourceTopic | undefined =>
  (Object.entries(sourceEditions).find(([, edition]) =>
    [edition.shared, edition.ts, edition.rust].includes(path)
  )?.[0] as SourceTopic | undefined)

export const sourcePathFor = (topic: SourceTopic, language: Language): string =>
  sourceEditions[topic][language] ?? sourceEditions[topic].shared
