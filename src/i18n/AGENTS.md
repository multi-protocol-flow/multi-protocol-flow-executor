# i18n 知识库

## OVERVIEW

typesafe-i18n v5.26.2 提供编译时类型安全的国际化方案。支持 zh-CN（baseLocale）+ en-US 双语言，翻译键在编译期类型推导，IDE 自动补全。同步预加载两个 locale（`index.ts` 中 `loadLocale` 调用），消除异步竞态。

## STRUCTURE

```
i18n/
├── en-US/                  # 英文翻译（11 个模块）
│   ├── common.ts, execution.ts, flow.ts, index.ts
│   ├── nodes.ts, panels.ts, settings.ts, shortcuts.ts
│   └── validation.ts, viewer.ts, welcome.ts
├── zh-CN/                  # 中文翻译（11 个模块，结构与 en-US 镜像）
├── i18n-types.ts           # 类型定义（自动生成，33110 行，编译推断所有翻译键）
├── i18n-util.ts            # 工具函数（typesafe-i18n 生成）
├── i18n-util.sync.ts       # 同步加载器
├── i18n-util.async.ts      # 异步加载器（本项目未使用）
├── i18n-react.tsx          # React context + TypesafeI18n Provider + useI18nContext
├── i18n-lazy.ts            # 懒加载辅助
├── custom-types.ts         # 自定义格式化类型
├── formatters.ts           # 日期/数字等格式化器
└── index.ts                # 入口：initI18n() + switchLocale() + re-exports
```

## WHERE TO LOOK

| 需求 | 位置 |
|------|------|
| 组件内获取翻译 | `getLL()` hook（import from `@/i18n` 或 typesafe-i18n） |
| 切换语言 | `switchLocale(newLocale)` from `@/i18n` |
| 新增翻译键 | `en-US/xxx.ts` + `zh-CN/xxx.ts` + `i18n-types.ts` 类型声明 |
| 翻译入口组件 | `I18nBridge` in `src/App.tsx` |

## CONVENTIONS

- **组件内用 `getLL()`，禁止用 `useI18nContext()`**。`useI18nContext()` 只在 `App.tsx` 的 `I18nBridge` 中使用——它负责将 locale 变化下推到 React tree。非 Bridge 组件用 `getLL()` 确保随 context 重渲染。
- **新增翻译键三步走**：`en-US/<module>.ts` + `zh-CN/<module>.ts` + `i18n-types.ts`。缺一不可，否则编译或运行时出错。
- **同步预加载**：`index.ts` 中 `loadLocale('zh-CN')` + `loadLocale('en-US')` 确保切换无延迟。新增 locale 必须同步注册。
- **类型安全**：`i18n-types.ts` 由 typesafe-i18n 生成，手动编辑无效。通过 `npm run typecheck` 验证翻译类型匹配。

## ANTI-PATTERNS

- **非 Bridge 组件使用 `useI18nContext()`**：导致 translation functions 未随 locale 更新，页面不重渲染。
- **新增键未更新 `i18n-types.ts`**：TypeScript 编译失败或类型缺失。
- **只加一个 locale**：另一语言用户看到空字符串或类型错误。
- **直接修改自动生成文件**（`i18n-types.ts`、`i18n-util.ts`、`i18n-react.tsx`）：下次 typesafe-i18n 生成时覆盖丢失。
