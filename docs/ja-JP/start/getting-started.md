---
read_when:
  - ゼロからの初回セットアップ
  - 最短で動くチャットを始めたい
summary: ゼロから最初のCodexPlusClawチャットまで。
title: はじめに
x-i18n:
  generated_at: "2026-03-11T00:00:00Z"
  model: gpt-5
  provider: openai
  source_path: start/getting-started.md
---

# はじめに

目標：最小限のセットアップで最初のCodexPlusClawチャットを動かすこと。

## 前提条件

- Node 22 以上

## クイックセットアップ

```bash
curl -fsSL https://openclaw.ai/install.sh | bash
openclaw setup --one-click
openclaw gateway status
openclaw dashboard
```

`setup --one-click` は、互換性のあるCodex CLIの導入または更新、ローカル
Gatewayの設定、ワークスペースとskillsディレクトリの準備、ヘルスチェック、
ダッシュボード起動までをまとめて行います。

手動またはリモートのウィザードが必要なら、`openclaw onboard` を使います。

## さらに詳しく

- ワンクリック設定の詳細：[setup](/cli/setup)
- 手動ウィザード：[wizard](/start/wizard)
- Control UI：[control-ui](/web/control-ui)
