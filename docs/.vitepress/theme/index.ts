import DefaultTheme from 'vitepress/theme'
import { dataSymbol, useData, useRoute } from 'vitepress'
import { computed, h, provide, ref } from 'vue'
import type { DefaultTheme as DefaultThemeTypes } from 'vitepress/theme'
import SourceVersionSwitch from './SourceVersionSwitch.vue'
import { sourceEditions, sourcePathFor, sourceVersionStateKey, type Language } from './sourceVersionState'
import './custom.css'

const topicForSharedPath = (path: string) => {
  if (path === sourceEditions['reading-00'].shared) return 'reading-00' as const
  if (path === sourceEditions['chapter-01'].shared) return 'chapter-01' as const
  if (path === sourceEditions['chapter-model-hello'].shared) return 'chapter-model-hello' as const
  return undefined
}

export default {
  ...DefaultTheme,
  Layout: {
    setup() {
      const selectedLanguage = ref<Language>('ts')
      provide(sourceVersionStateKey, selectedLanguage)
      const data = useData()
      const route = useRoute()

      const stripBase = (path: string) => {
        const base = data.site.value.base || '/'
        return (base !== '/' && path.startsWith(base) ? path.slice(base.length - 1) : path).replace(/\/$/, '') || '/'
      }
      const transformSidebar = (items: DefaultThemeTypes.SidebarItem[]): DefaultThemeTypes.SidebarItem[] => items.map((item) => {
        const topic = item.link && topicForSharedPath(stripBase(item.link))
        const currentPath = stripBase(route.path)
        const link = topic
          ? (currentPath === sourceEditions[topic].shared ? sourceEditions[topic].shared : sourcePathFor(topic, selectedLanguage.value))
          : item.link
        return {
          ...item,
          ...(link ? { link } : {}),
          ...(item.items ? { items: transformSidebar(item.items) } : {})
        }
      })
      const resolvedTheme = computed(() => ({
        ...data.theme.value,
        sidebar: Array.isArray(data.theme.value.sidebar)
          ? transformSidebar(data.theme.value.sidebar)
          : Object.fromEntries(Object.entries(data.theme.value.sidebar ?? {}).map(([key, items]) => [key, transformSidebar(items as DefaultThemeTypes.SidebarItem[])]))
      }))
      provide(dataSymbol, { ...data, theme: resolvedTheme })
      return () => h(DefaultTheme.Layout, null, {
        'layout-bottom': () => h(SourceVersionSwitch, { variant: 'floating' })
      })
    }
  }
}
