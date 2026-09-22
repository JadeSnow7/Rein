<script setup lang="ts">
import { computed, inject, onMounted, ref, watch, type Ref } from 'vue'
import { getScrollOffset, useData, useRoute, useRouter, withBase } from 'vitepress'
import { resolveAnchorHash, sourceEditions, sourceVersionStateKey, sourceTopicForPath, type Language } from './sourceVersionState'

const { variant = 'floating' } = defineProps<{ variant?: 'floating' }>()

const selectedLanguage = inject<Ref<Language>>(sourceVersionStateKey, ref<Language>('ts'))

const preferenceKey = 'rein:source-language'
const languageLabels: Record<Language, string> = { ts: 'TS', rust: 'Rust' }
const route = useRoute()
const router = useRouter()
const { site } = useData()
const ready = ref(false)

const pathWithoutBase = (path: string) => {
  const base = site.value.base || '/'
  const withoutBase = base !== '/' && path.startsWith(base)
    ? path.slice(base.length - 1)
    : path
  return withoutBase.replace(/\/$/, '') || '/'
}

const currentPath = computed(() => pathWithoutBase(route.path.split('#')[0]))
const topic = computed(() => sourceTopicForPath(currentPath.value))
const pair = computed(() => topic.value ? sourceEditions[topic.value] : undefined)
const explicitLanguage = computed<Language | undefined>(() => {
  if (!pair.value) return undefined
  if (pair.value.rust && currentPath.value === pair.value.rust) return 'rust'
  if (currentPath.value === pair.value.ts) return 'ts'
  return undefined
})
const isPublicContent = computed(() => Boolean(pair.value && !explicitLanguage.value))
const isGenericContent = computed(() => !pair.value)
const rustAvailable = computed(() => !pair.value || Boolean(pair.value.rust))
const statusText = computed(() => {
  if (!pair.value) return '公共内容 · 语言偏好仅影响有对应版本的主题'
  if (topic.value === 'chapter-01' && !rustAvailable.value) return isPublicContent.value ? '公共内容 · Rust 待补齐' : '当前源码：TS · Rust 待补齐'
  if (isPublicContent.value) return '公共内容 · 可进入语言版'
  return `当前源码：${languageLabels[explicitLanguage.value ?? selectedLanguage.value]}`
})

const readPreference = (): Language => {
  try {
    const value = window.localStorage.getItem(preferenceKey)
    return value === 'rust' ? 'rust' : 'ts'
  } catch {
    return 'ts'
  }
}

const savePreference = (value: Language) => {
  try {
    window.localStorage.setItem(preferenceKey, value)
  } catch {
    // Storage is an optional convenience; navigation must keep working without it.
  }
}

const syncLanguageFromRoute = () => {
  if (explicitLanguage.value) {
    selectedLanguage.value = explicitLanguage.value
    savePreference(selectedLanguage.value)
  } else if (!ready.value) {
    selectedLanguage.value = readPreference()
  }
}

const targetHash = () => {
  if (typeof window === 'undefined' || !pair.value) return ''
  const headings = Array.from(document.querySelectorAll<HTMLElement>('.vp-doc h2[id], .vp-doc h3[id], .vp-doc span[id]'))
    .filter((element) => element.matches('h2, h3') || pair.value.commonHashes.includes(element.id))
  if (!headings.length) return ''
  const currentPosition = window.scrollY + getScrollOffset() + 4
  const visibleHeading = headings
    .filter((element) => element.getBoundingClientRect().top + window.scrollY <= currentPosition)
    .at(-1)
  const visibleIndex = visibleHeading ? headings.indexOf(visibleHeading) : -1
  const parentHeading = visibleIndex >= 0
    ? headings.slice(0, visibleIndex + 1).reverse().find((element) => element.tagName.toLowerCase() === 'h2')
    : undefined
  return resolveAnchorHash(visibleHeading?.id, pair.value, parentHeading?.id)
}

const selectLanguage = async (next: Language) => {
  selectedLanguage.value = next
  savePreference(next)
  if (!pair.value) return
  const target = pair.value[next]
  if (!target || target === currentPath.value) {
    return
  }
  await router.go(withBase(`${target}${targetHash()}`))
}

onMounted(() => {
  syncLanguageFromRoute()
  ready.value = true
})

watch(() => route.path, syncLanguageFromRoute)
</script>

<template>
  <div v-if="pair?.rust" class="source-version-switch" :class="`source-version-switch--${variant}`" :aria-label="statusText">
    <div class="source-version-switch__buttons" role="group" aria-label="切换源码语言">
      <button
        v-for="option in (['ts', 'rust'] as Language[])"
        :key="option"
        type="button"
        :disabled="option === 'rust' && !rustAvailable"
        :class="{ active: isGenericContent ? selectedLanguage === option : explicitLanguage === option, unavailable: option === 'rust' && !rustAvailable }"
        :aria-pressed="isGenericContent ? selectedLanguage === option : explicitLanguage === option"
        @click="selectLanguage(option)"
      >{{ option === 'rust' && !rustAvailable ? 'Rust 待补齐' : languageLabels[option] }}</button>
    </div>
    <span class="source-version-switch__status" role="status" aria-live="polite">{{ statusText }}</span>
  </div>
</template>
