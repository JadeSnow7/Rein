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
        { text: 'Python 主线 · 从最小请求到建议', collapsed: false, items: [
          { text: '00 最小 Agent', link: '/chapters/minimal-agent.html' },
          { text: '01 Python 模型调用', link: '/chapters/python-model-call.html' },
          { text: '02 Python 读取文件', link: '/chapters/python-file-read.html' },
          { text: '03 Python 生成建议', link: '/chapters/python-suggestions.html' }
        ] },
        { text: '旧版第一部分 · 让模型完成一个小任务', collapsed: false, items: [
          { text: '00 绪论', link: '/chapters/task-map.html' }, { text: '阅读 0 基础知识', link: '/readings/00.html' }, { text: '01 HelloWorld——从模型调用开始', link: '/chapters/model-hello.html' }, { text: '02 任务与成功标准', link: '/chapters/task-spec.html' }, { text: '03 工具调用', link: '/chapters/tool-roundtrip.html' }, { text: '阶段汇总 1', link: '/milestones/evidence-qa.html' }
        ] },
        { text: '第二部分 · 建立可控的混合运行时', collapsed: true, items: [
          { text: '04 模型接口 · 后续章节待发布', link: '/chapters/provider-adapter.html' }, { text: '05 Rust 迁移 · 后续章节待发布', link: '/chapters/rust-migration.html' }, { text: '06 核心 Agent Loop · 后续章节待发布', link: '/chapters/agent-loop.html' }, { text: '07–09 后续章节待发布' }, { text: '阶段汇总 2 · 后续章节待发布' }
        ] },
        { text: '第三部分 · 让回答有依据、可检查', collapsed: true, items: [
          { text: '10–13 后续章节待发布' }, { text: '阶段汇总 3 · 后续章节待发布' }
        ] },
        { text: '第四部分 · 让修改可审查、可验证', collapsed: true, items: [
          { text: '14–17 后续章节待发布' }, { text: '阶段汇总 4 · 后续章节待发布' }
        ] },
        { text: '第五部分 · 可选能力分支与结项', collapsed: true, items: [
          { text: '18–24 后续章节待发布' }, { text: '阶段汇总 5 · 后续章节待发布' }
        ] },
        { text: '补充入口', collapsed: true, items: [
          { text: '阅读材料与附录 · 后续章节待发布' }, { text: '历史页面', link: '/history.html' }
        ] }
      ]
    }
  }
})
