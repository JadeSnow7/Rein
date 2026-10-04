import { defineConfig } from 'vitepress'
import book from '../../book/chapters.json'

const chapterLink = (chapter: typeof book.chapters[number] | undefined) => chapter ? { text: `${String(chapter.order).padStart(2, '0')} ${chapter.title}`, link: chapter.route } : false
const milestoneLink = (milestone: typeof book.milestones[number]) => ({ text: `阶段汇总 · ${milestone.title}`, link: milestone.path })
const currentSidebar = [
  { text: '开始阅读', items: [{ text: '关于本书', link: '/about.html' }, { text: '全书目录', link: '/toc.html' }, { text: '开放说明', link: '/access.html' }] },
  ...book.parts.map((part) => ({
    text: `${part.title}（${String(part.range[0]).padStart(2, '0')}–${String(part.range[1]).padStart(2, '0')}）`,
    collapsed: part.id !== 'part-1',
    items: [
      ...book.chapters.filter((chapter) => chapter.part === part.id).map(chapterLink),
      ...book.milestones.filter((milestone) => book.chapters[milestone.after_order]?.part === part.id).map(milestoneLink)
    ]
  })),
  { text: '历史材料', collapsed: true, items: [{ text: '旧版五部分目录与页面', link: '/history.html' }, { text: '阅读材料', link: '/readings/00.html' }, { text: '实现对照与深入讨论', link: '/appendices/a1.html' }] }
]

const chapterNavigation = (pageData: { relativePath: string; frontmatter: Record<string, unknown> }) => {
  const path = '/' + pageData.relativePath.replace(/\.md$/, '.html')
  const index = book.chapters.findIndex((chapter) => chapter.route.split('#')[0] === path)
  const milestone = book.milestones.find((item) => item.path === path)
  pageData.frontmatter ||= {}
  // Roadmap pages contain several planned chapters and are overview pages.
  // Chapter arrows belong to dedicated chapter pages only.
  if (path.startsWith('/roadmap/')) return
  if (milestone) {
    const chapterIndex = milestone.after_order
    pageData.frontmatter.prev = chapterLink(book.chapters[chapterIndex])
    pageData.frontmatter.next = chapterLink(book.chapters[chapterIndex + 1])
  } else if (index >= 0) {
    const prior = book.milestones.find((item) => item.after_order === index - 1)
    const next = book.milestones.find((item) => item.after_order === index)
    pageData.frontmatter.prev = prior ? milestoneLink(prior) : chapterLink(book.chapters[index - 1])
    pageData.frontmatter.next = next ? milestoneLink(next) : chapterLink(book.chapters[index + 1])
  }
}

export default defineConfig({
  lang: 'zh-CN', title: 'Rein', description: '从零构建 Agent：人定设计，AI 编码的 Agent Harness 工程实践', base: '/Rein/', cleanUrls: false, lastUpdated: true,
  transformPageData: chapterNavigation,
  themeConfig: { logo: '/mark.svg', siteTitle: 'REIN / AGENT ENGINEERING', nav: [{ text: '首页', link: '/' }, { text: '阅读指南', link: '/about.html' }, { text: '全书目录', link: '/toc.html' }, { text: '开放说明', link: '/access.html' }], outline: { level: [2, 3], label: '本页内容' }, search: { provider: 'local' }, socialLinks: [{ icon: 'github', link: 'https://github.com/JadeSnow7/Rein' }], footer: { message: '一条可运行、可解释、可验证的 Agent 学习路线', copyright: '© JadeSnow7' }, sidebar: { '/': currentSidebar } }
})
