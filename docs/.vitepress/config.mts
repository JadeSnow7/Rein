import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'Rein',
  description: '从零手写一个 Agent Harness',
  base: '/Rein/',
  cleanUrls: false,
  lastUpdated: true,
  themeConfig: {
    logo: '/mark.svg',
    siteTitle: 'REIN / AGENT ENGINEERING',
    nav: [
      { text: '首页', link: '/' },
      { text: '阅读指南', link: '/about.html' },
      { text: '全书目录', link: '/toc.html' }
    ],
    outline: { level: [2, 3], label: '本页内容' },
    search: { provider: 'local' },
    socialLinks: [{ icon: 'github', link: 'https://github.com/JadeSnow7/Rein' }],
    footer: {
      message: '一条可运行、可解释、可验证的 Agent 学习路线',
      copyright: '© JadeSnow7'
    },
    sidebar: {
      '/': [
        { text: '开始阅读', items: [{ text: '关于本书', link: '/about.html' }, { text: '全书目录', link: '/toc.html' }] },
        { text: '第一阶段 · 直觉', collapsed: false, items: [
          { text: '00 绪论', link: '/toc.html#ch00' }, { text: '阅读材料 0', link: '/toc.html#chread0' }, { text: '01 从模型调用开始', link: '/toc.html#ch01' }, { text: '02 写一份合格的提示词', link: '/toc.html#ch02' }, { text: '阅读材料 1', link: '/toc.html#chread1' }, { text: '03 工具调用', link: '/toc.html#ch03' }
        ] },
        { text: '第二阶段 · 循环', collapsed: true, items: [
          { text: '04 统一协议', link: '/toc.html#ch04' }, { text: '05 核心 Agent Loop', link: '/toc.html#ch05' }, { text: '06 循环控制', link: '/toc.html#ch06' }, { text: '阅读材料 2', link: '/toc.html#chread2' }, { text: '阅读材料 3', link: '/toc.html#chread3' }
        ] },
        { text: '第三阶段 · 可靠性', collapsed: true, items: [
          { text: '07 上下文管理', link: '/toc.html#ch07' }, { text: '08 上下文管理方法', link: '/toc.html#ch08' }, { text: '09 验收条件与验证器', link: '/toc.html#ch09' }, { text: '10 失败反馈', link: '/toc.html#ch10' }, { text: '11 防御性编程', link: '/toc.html#ch11' }, { text: '12 权限管理', link: '/toc.html#ch12' }
        ] },
        { text: '第四阶段 · 应用', collapsed: true, items: [
          { text: '13 计划与任务审查', link: '/toc.html#ch13' }, { text: '14 多智能体协作', link: '/toc.html#ch14' }, { text: '15 外部扩展', link: '/toc.html#ch15' }, { text: '16 垂直领域设计方法论', link: '/toc.html#ch16' }
        ] }
      ]
    }
  }
})
