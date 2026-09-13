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
      { text: '全书目录', link: '/toc.html' },
      { text: '开放说明', link: '/access.html' }
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
        { text: '开始阅读', items: [{ text: '关于本书', link: '/about.html' }, { text: '全书目录', link: '/toc.html' }, { text: '开放说明', link: '/access.html' }] },
        { text: '第一阶段 · 直觉', collapsed: false, items: [
          { text: '00 绪论 · 待撰写', link: '/chapters/00.html' }, { text: '阅读 0 基础知识', link: '/readings/00.html' }, { text: 'HelloWorld——从模型调用开始', link: '/chapters/01.html' }, { text: '02 写一份合格的提示词 · 待撰写', link: '/chapters/02.html' }, { text: '阅读材料 1 · 待撰写', link: '/readings/01.html' }, { text: '03 工具调用 · 待撰写', link: '/chapters/03.html' }, { text: '小结 · 待撰写', link: '/milestones/01.html' }
        ] },
        { text: '第二阶段 · 循环', collapsed: true, items: [
          { text: '04 统一协议 · 待撰写', link: '/chapters/04.html' }, { text: '05 核心 Agent Loop · 待撰写', link: '/chapters/05.html' }, { text: '06 循环控制 · 待撰写', link: '/chapters/06.html' }, { text: '阅读材料 2 · 待撰写', link: '/readings/02.html' }, { text: '阅读材料 3 · 待撰写', link: '/readings/03.html' }, { text: '阶段汇总 1 · 待撰写', link: '/milestones/02.html' }
        ] },
        { text: '第三阶段 · 可靠性', collapsed: true, items: [
          { text: '07 上下文管理 · 待撰写', link: '/chapters/07.html' }, { text: '08 上下文管理方法 · 待撰写', link: '/chapters/08.html' }, { text: '09 验收条件与验证器 · 待撰写', link: '/chapters/09.html' }, { text: '10 失败反馈 · 待撰写', link: '/chapters/10.html' }, { text: '11 防御性编程 · 待撰写', link: '/chapters/11.html' }, { text: '12 权限管理 · 待撰写', link: '/chapters/12.html' }, { text: '阶段汇总 2 · 待撰写', link: '/milestones/03.html' }
        ] },
        { text: '第四阶段 · 应用', collapsed: true, items: [
          { text: '13 计划与任务审查 · 待撰写', link: '/chapters/13.html' }, { text: '14 多智能体协作 · 待撰写', link: '/chapters/14.html' }, { text: '15 外部扩展 · 待撰写', link: '/chapters/15.html' }, { text: '16 垂直领域设计方法论 · 待撰写', link: '/chapters/16.html' }, { text: '阶段汇总 3 · 待撰写', link: '/milestones/04.html' }
        ] },
        { text: '实现对照与深入讨论', collapsed: true, items: [
          { text: 'A.1 行为约定与实现边界 · 待撰写', link: '/appendices/a1.html' }, { text: 'A.2 运行状态 · 待撰写', link: '/appendices/a2.html' }, { text: 'A.3 进程与取消 · 待撰写', link: '/appendices/a3.html' }, { text: 'A.4 崩溃恢复 · 待撰写', link: '/appendices/a4.html' }, { text: 'A.5 并发汇总 · 待撰写', link: '/appendices/a5.html' }
        ] }
      ]
    }
  }
})
