# i18n

**Layer:** 前端服务 | ~12 文件 | baseLocale: zh-CN

## 概述

`typesafe-i18n` v5.26.2 国际化方案，编译时类型安全。支持 zh-CN / en-US 双语言，翻译按领域分 7 个模块。

## 结构

```
src/i18n/
├── zh-CN/              # 中文翻译 (7 模块)
├── en-US/              # 英文翻译 (7 模块)
├── i18n-types.ts       # 类型定义 (auto-generated，手动维护)
├── i18n-util.ts        # 工具函数 (auto-generated)
├── i18n-react.tsx      # React Context (auto-generated，仅 I18nBridge 使用)
├── i18n-lazy.ts        # getLL() — 模块级非响应式翻译获取
├── index.ts            # initI18n / switchLocale / TypesafeI18n 导出
├── custom-types.ts     # 自定义类型扩展
├── formatters.ts       # 自定义格式化器
└── viewer-translations.ts # 查看器翻译映射
```

## WHERE TO LOOK

| 任务 | 文件 |
|------|------|
| 获取翻译文本 | `i18n-lazy.ts` → `getLL()` |
| 切换语言 | `index.ts` → `switchLocale()` |
| 新增翻译 key | `zh-CN/<module>.ts` + `en-US/<module>.ts` + `i18n-types.ts` |
| 新增语言 | `index.ts` 预加载 + `.typesafe-i18n.json` + locale 目录 |

## CONVENTIONS

- **获取翻译**: 用 `getLL()`，不用 `useI18nContext()` / `contextLL`
- **原因**: `getLL()` 从 localStorage 读取 locale，无响应式依赖；`useI18nContext()` 与项目 locale 切换机制不兼容，切换后仍拿旧 locale
- **例外**: `App.tsx` 中的 `I18nBridge` 组件是唯一使用 `useI18nContext()` 的地方，作为 locale 切换触发点
- **预加载**: `index.ts` 同步 `loadLocale('zh-CN')` + `loadLocale('en-US')` 消除竞态
- **新增 key**: 编辑 `zh-CN/<module>.ts` + `en-US/<module>.ts` + `i18n-types.ts` 对应类型块。`i18n-types.ts` 虽有 "auto-generated" 注释，但项目无自动生成脚本，需手动同步

## ANTI-PATTERNS

- 非 Bridge 组件使用 `useI18nContext()` — 翻译不随 locale 切换更新
- 跳过 `i18n-types.ts` 类型更新 — 新增翻译 key 后类型缺失导致编译错误
