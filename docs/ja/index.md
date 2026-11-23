---
layout: home
title: Agent Project
markdownStyles: false

landing:
  lanes: [未着手, 進行中, 完了]
  copy:
    label: コマンドをコピー
    done: コピーしました
  hero:
    title: "人が回さなくても、\n大きな仕事が進む。"
    lead: Agent Projectは、一人でプロダクトを育てる人が、大きな仕事を計画から検証までCodexやClaude Codeに任せるためのツールです。計画と進み具合はカンバンに並び、あなたの判断が要る質問はメッセージに届きます。この2つがあれば、付きっきりにならなくてもプロジェクトは進みます。
    install: brew install fooqoo/tap/agpj
    primary: { text: はじめる, link: /ja/guide/getting-started }
    secondary: { text: GitHub, link: https://github.com/FooQoo/agent-project }
    meta: macOS · Claude Code / Codex · MIT License
    videoLabel: デモ動画。AI に依頼で頼むと、エージェントがタスクに分けて質問し、回答すると作業を進めて、完了まで進む
  pm:
    eyebrow: エージェントのためのプロジェクトマネジメント
    title: 仕事の段取りを任せられるから、エージェントが自走できる。
    lead: エージェントは強くなりましたが、長い仕事では、途中で迷う、止まる、終わっていないのに終えてしまう、ということが起こります。そこでAgent Projectは、長い仕事を最後まで進めるための4つの決まりを、エージェントに持たせます。
    items:
      - { icon: kanban, tone: todo, title: 分けて、順番を決める, text: 目標がエピックとタスクに分かれて順に並び、次に進められる作業を、進行表がエージェントに伝えます。 }
      - { icon: list-checks, tone: done, title: 終わりを先に決める, text: タスクごとに、作業の前に完了条件を決めます。すべての条件を確かめたときだけ、自動で完了に移ります。 }
      - { icon: message-circle, tone: doing, title: 決める前に聞く, text: 意図が分からない・好みで決まる・取り返しがつかない・範囲が広がる、のときは、エージェントが止まり、選択肢をつけてあなたに聞きます。 }
      - { icon: file-text, tone: pending, title: やったことを残す, text: 履歴と結果に、やったことと決めたことが残るので、次のセッションや別のエージェントが、そこから始められます。 }
  problem:
    eyebrow: なぜAgent Projectか
    title: 質問が、ログに流れていかない。
    lead: ターミナルだけだと、あなたに必要な質問は流れるログに埋もれ、計画や条件はチャットの中にしかありません。Agent Projectでは、計画と進み具合はカンバンに、あなたへの質問はメッセージに残るので、エージェントが呼ぶまで離れていられます。
    terminal:
      label: ターミナルの中だけ
      caption: 質問は、流れていくログの途中に埋もれてしまう。
      lines:
        - { text: Read docs/plan.md }
        - { text: Edit src/pages/pricing.tsx }
        - { text: +42 −8, sub: true }
        - { text: Bash pnpm test }
        - { text: ✓ 128 passed, sub: true }
        - { text: "公開方法を決めてください: 1) 先にプレビュー 2) 一般公開", question: true }
        - { text: Edit src/pages/index.tsx }
        - { text: +12 −3, sub: true }
        - { text: Bash pnpm build }
        - { text: built in 4.2s, sub: true }
        - { text: Edit docs/release-notes.md }
    board:
      label: Agent Projectで
      caption: 回答待のタスクと質問が、列の中ですぐ目に入る。
      alt: 進行中の列で回答待のタスクに、エージェントの質問と選択肢が表示されたカンバン
    questions:
      - { title: 何が止まっている？, text: 回答待のタスクは色が変わり、列の中で目立ちます。 }
      - { title: 何に答えればいい？, text: 質問と選択肢が、タスクの上にそのまま出ます。 }
      - { title: どこまで終わった？, text: 完了条件の達成数と履歴が、タスクごとに残ります。 }
  loop:
    eyebrow: 進み方
    title: 計画を頼む。開始する。答える。受け取る。
    lead: やりたいことをエージェントに伝えると、完了条件つきの計画がバックログに置かれ、決められないことが質問されます。その計画を未着手へ移すと、エージェントが順に作業を始め、あなたが答えるのは止まって聞かれたときだけです。戻ってくる結果は、完了条件に照らして確かめられています。
    actors: { ai: AI, human: あなた, auto: 自動 }
    lanes:
      - title: 計画
        steps:
          - { actor: human, title: AIに計画を頼む, text: やりたいことを、普通の言葉でエージェントに伝えます。 }
          - { actor: ai, title: 計画, text: エピックとタスクを、完了条件つきでバックログに置き、自分では決められないことを質問します。 }
      - title: 開始
        steps:
          - { actor: human, title: 開始する, text: バックログの計画を未着手へ移すと、エージェントが作業を始めます。移すまで、エージェントは計画に手を付けません。 }
      - title: 作業
        steps:
          - { actor: ai, title: 作業, text: 順にタスクを進めます。止まるのは、意図が分からない・好みで決まる・取り返しがつかない・範囲が広がる、のときだけです。 }
          - { actor: human, title: 途中で答える, text: メッセージから、PCでもスマホでも答えられます。チャットで答えてもかまいません。回答は一度だけ記録され、エージェントが作業を続けます。 }
      - title: 結果
        steps:
          - { actor: auto, title: 結果を受け取る, text: 確かめた条件がすべてそろうと、タスクは自動で完了へ移ります。やったことと決めたことは、履歴と結果に残ります。 }
  features:
    eyebrow: 2つの画面
    title: カンバンで見守り、メッセージで答える。
    previewLabel: パブリックプレビュー
    items:
      - image: board
        title: カンバン
        text: 計画と仕事が、バックログ・未着手・進行中・完了の列に並びます。どこまで進み、どのタスクが止まっているかがひと目でわかり、カードを開けば完了条件と履歴を確かめられます。
        link: { text: カンバンの使い方, link: /ja/guide/watch/board }
        alt: タスク内の回答欄、完了条件の達成数、詳細パネルを表示したPC版カンバン
      - image: messages
        title: メッセージ
        text: エージェントからの質問と、あなたからの依頼が一か所に集まります。選択肢を選んで答えると、エージェントがその回答を受け取って作業を続けます。
        link: { text: 判断依頼, link: /ja/guide/delegate/decisions }
        alt: AIの質問・あなたの依頼・サマリーのタブがあるメッセージの一覧
      - image: epic
        title: エピックと進行表
        text: エピックが計画とタスクの順序を持ち、進行表が、いま進められる作業をエージェントに返します。計画がどこまで進んだかも見えます。
        link: { text: エピックと進行表, link: /ja/guide/watch/epics }
        alt: タスクの進行表とMarkdown本文を持つエピック編集画面
      - image: mobile
        title: どこからでも確認
        text: 席を離れていても、スマホやタブレットでカンバンとメッセージを開き、仕事を見守り、エージェントの質問に答えられます。ホーム画面に追加すればアプリのように使え、成果などの通知もスマホに届きます。
        link: { text: リモートアクセス, link: /ja/guide/watch/remote-access }
        tabletAlt: "タブレットで開いた同じカンバン。未着手・進行中・完了の列が並び、回答待のタスクと「質問に回答する」ボタンが見える"
        alts:
          - 回答待のタスクと「質問に回答する」ボタンが見える、リモートアクセスのカードの一覧
          - 選択肢を選び、送信ボタンが出ているリモートアクセスの質問
          - リモートアクセスの下部のドロワーで、完了条件のタブを開いたタスク
      - image: docs
        title: プロジェクトのドキュメント
        text: プロジェクトのdocs/にある設計書や仕様を、カンバンから開いて読めます。リリースの記録も自動で残ります。版ごとにリリース文書が用意され、その版で予定した作業、確かめ終えた作業、残った作業がまとめられます。
        link: { text: プロジェクトのドキュメント, link: /ja/guide/watch/docs }
        preview: /ja/guide/feature-stages#docs
        alt: ファイル一覧とMarkdownの文書を表示した「ドキュメント」ダイアログ
  agents:
    eyebrow: 連携
    title: いつものエージェントから、そのまま。
    lead: agpj setupが、Claude CodeとCodexにSkillとHookを含むAgent Projectプラグインを導入します。同じ仕事のモデルを、どちらでも使えます。エージェントは進行表を読み、仕事を計画し、質問して回答を待ちます。
    supportedLabel: 対応エージェント
    supported:
      - { text: Claude Code, link: https://www.claude.com/product/claude-code }
      - { text: Codex, link: https://developers.openai.com/codex }
    trademark: Claude CodeはAnthropic、CodexはOpenAIの製品で、各名称は各社の商標です。Agent Projectは独立したプロジェクトで、両社と提携しておらず、推奨も受けていません。
    command: agpj setup
    terminal:
      label: ターミナルで
      caption: エージェントを選んで確認するだけで、プラグインが入ります。
      lines:
        - { kind: command, text: $ agpj setup }
        - { kind: frame, text: "┌  agpj setup" }
        - { kind: question, text: "◇  Which agent do you use with Agent Project?" }
        - { kind: answer, text: "│  Both" }
        - { kind: question, text: "◇  Apply these changes? …" }
        - { kind: answer, text: "│  Yes" }
        - { kind: result, text: "◆  Codex: updated" }
        - { kind: result, text: "◆  Claude Code: updated" }
    link: { text: エージェントとの連携, link: /ja/guide/delegate/agents }
  trust:
    title: 手元で動き、データはあなたのもの。
    items:
      - { title: ローカルで動く, text: サーバーは127.0.0.1だけで待ち受けます。 }
      - { title: Markdownで残る, text: タスクはTASK.md、エピックはEPIC.md。直接編集もできます。 }
      - { title: OSS（MIT）, text: ソースコードをGitHubで公開しています。 }
      - { title: アカウント不要, text: 登録もクラウドサービスも要りません。 }
  start:
    eyebrow: はじめ方
    title: 3つの手順で始められます。
    lead: macOS 15以降とHomebrew、Claude CodeかCodexがあれば始められます。
    steps:
      - { title: インストールする, commands: [brew install fooqoo/tap/agpj] }
      - { title: エージェントとプロジェクトを設定する, commands: [agpj setup, 'agpj project create "My project"'] }
      - { title: カンバンを開く, commands: [agpj serve] }
    note: "ドキュメントや常駐エージェントなど、一部の機能はパブリックプレビューです。詳しくは"
    noteLinks:
      - { text: 機能の提供段階, link: /ja/guide/feature-stages }
    noteJoin: と
    noteEnd: を参照してください。
    primary: { text: インストールと起動を読む, link: /ja/guide/getting-started }
    secondary: { text: GitHub, link: https://github.com/FooQoo/agent-project }
---

<LandingPage />
