---
layout: home
title: Agent Project
markdownStyles: false

landing:
  lanes: [Todo, Doing, Done]
  copy:
    label: Copy command
    done: Copied
  hero:
    title: "Big work moves on\nwithout you driving it."
    lead: Agent Project lets you hand off substantial work to Codex or Claude Code when you’re building a product on your own. Your agent breaks the work into tasks, puts them in order, and sets acceptance conditions before starting. It asks for your input when a decision needs you. The Kanban board shows progress and how the results met those conditions.
    install: brew install fooqoo/tap/agpj
    primary: { text: Get started, link: /guide/getting-started }
    secondary: { text: GitHub, link: https://github.com/FooQoo/agent-project }
    meta: macOS · Claude Code / Codex · MIT License
    videoLabel: "Demo: you send a request with Ask AI, the agent turns it into tasks and asks a question, you answer, and the agent works until the task is done"
  pm:
    eyebrow: Project management for agents
    title: Agents run on their own when the board manages the project.
    lead: A strong agent still loses its way in a long job. It wanders, stops, or calls the work done too early. The board gives it four things a project manager provides. Agent Project is for people building a product alone with Codex or Claude Code.
    items:
      - { icon: kanban, tone: todo, title: Break down and order, text: The goal becomes epics and tasks in order, and the progress table tells the agent what can move next. }
      - { icon: list-checks, tone: done, title: Decide what done means, text: Each task has acceptance conditions set before the work starts. It reaches Done on its own only when every condition is verified. }
      - { icon: message-circle, tone: doing, title: Ask before deciding, text: "When your intent is unclear, your preference decides, an action cannot be undone, or the work grows, the agent stops and asks you, with options." }
      - { icon: file-text, tone: pending, title: Keep what it did, text: History and results record what the agent did and decided, so the next session or agent starts from there. }
  problem:
    eyebrow: Why a board
    title: The question should not scroll past.
    lead: In a terminal alone, the one question that needs you is buried in the log, and the plan and the conditions live only in the chat. On the board they stand where you can see them, so you can step away until the agent needs you.
    terminal:
      label: Only in the terminal
      caption: The question scrolls past in the middle of the log.
      lines:
        - { text: Read docs/plan.md }
        - { text: Edit src/pages/pricing.tsx }
        - { text: +42 −8, sub: true }
        - { text: Bash pnpm test }
        - { text: ✓ 128 passed, sub: true }
        - { text: "Choose how to publish: 1) preview first 2) publish", question: true }
        - { text: Edit src/pages/index.tsx }
        - { text: +12 −3, sub: true }
        - { text: Bash pnpm build }
        - { text: built in 4.2s, sub: true }
        - { text: Edit docs/release-notes.md }
    board:
      label: Shared on the board
      caption: A waiting task and its question stand out in the lane.
      alt: The board with a waiting task in Doing that shows the agent's question and its options
    questions:
      - { title: What is stuck?, text: Waiting tasks change color and stand out in their lane. }
      - { title: What needs my answer?, text: The question and its options appear right on the task. }
      - { title: How far along is it?, text: Each task keeps its acceptance count and its history. }
  loop:
    eyebrow: How it goes
    title: Ask AI to plan. Approve. Answer. Receive.
    lead: Tell your agent what you want. It puts a plan on the board with acceptance conditions and asks what it cannot settle. Once you approve the start, it works in order, and you answer only when it stops for you. What comes back is checked against the conditions.
    actors: { ai: AI, human: You, auto: Auto }
    lanes:
      - title: Plan
        steps:
          - { actor: human, title: Ask AI to plan, text: Tell your agent what you want, in plain words. }
          - { actor: ai, title: Plan, text: Puts epics and tasks on the board, each with acceptance conditions, and asks the questions it cannot settle. }
      - title: Start
        steps:
          - { actor: human, title: Approve the start, text: Answer the questions and send the plan to Todo. A task does not start while one of its questions is open. }
      - title: Work
        steps:
          - { actor: ai, title: Work, text: "Works through the tasks in order. It stops only when your intent is unclear, your preference decides, an action cannot be undone, or the work grows." }
          - { actor: human, title: Answer on the way, text: "Answer from the board, your phone, or the chat. The answer is recorded once, and the agent carries on." }
      - title: Result
        steps:
          - { actor: auto, title: Receive the result, text: A task moves to Done once every verified condition is met. What the agent did and decided stays in its history and results. }
  features:
    eyebrow: Monitors for the work
    title: Watch and step in from anywhere.
    previewLabel: Public preview
    items:
      - image: board
        title: The board
        text: Watch the work move through Todo, Doing, and Done. Step in on a card without stopping the agent, and answer its questions right on the task.
        link: { text: Using the board, link: /guide/watch/board }
        alt: The desktop board with a question on its task, acceptance counts, and the detail panel
      - image: epic
        title: Epics and progress
        text: An epic holds the plan and the order of its tasks. The progress table returns the work that can move next to your agent, and shows you how far the plan has come.
        link: { text: Epics and progress, link: /guide/watch/epics }
        alt: The epic editor with the task progress panel and the Markdown details
      - image: mobile
        title: Check from anywhere
        text: Away from your desk, open the board on your phone or tablet, watch the work, and answer the agent. Add it to the Home Screen and it works like an app, and notifications such as new results reach your phone.
        link: { text: Remote access, link: /guide/watch/remote-access }
        tabletAlt: "The same board opened on a tablet: the Todo, Doing, and Done lanes side by side, with a waiting task and its Answer the question button"
        alts:
          - The remote access card list, with a waiting task and its Answer the question button
          - A question on remote access with a choice picked and the Send button
          - A task open in the bottom drawer of remote access, on its Conditions tab
      - image: docs
        title: Project docs
        text: Open the design notes and specs in your project's docs/ folder right from the board.
        link: { text: Project docs, link: /guide/watch/docs }
        preview: /guide/feature-stages#docs
        alt: The Docs dialog with a file list and a rendered Markdown document
  agents:
    eyebrow: Integrations
    title: Works with the agents you already use.
    lead: agpj setup installs the Agent Project plugin, with the Skill and the Hook, for Claude Code and Codex. The same work model works across both. Your agent reads the progress table, plans the work, asks questions, and waits for your answer.
    supportedLabel: Supported agents
    supported:
      - { text: Claude Code, link: https://www.claude.com/product/claude-code }
      - { text: Codex, link: https://developers.openai.com/codex }
    trademark: Claude Code is a product of Anthropic and Codex is a product of OpenAI; their names are trademarks of their respective owners. Agent Project is an independent project and is not affiliated with or endorsed by either company.
    command: agpj setup
    terminal:
      label: In your terminal
      caption: Choose your agents and confirm, and the plugin is in place.
      lines:
        - { kind: command, text: $ agpj setup }
        - { kind: frame, text: "┌  agpj setup" }
        - { kind: question, text: "◇  Which agent do you use with Agent Project?" }
        - { kind: answer, text: "│  Both" }
        - { kind: question, text: "◇  Apply these changes? …" }
        - { kind: answer, text: "│  Yes" }
        - { kind: result, text: "◆  Codex: updated" }
        - { kind: result, text: "◆  Claude Code: updated" }
    link: { text: Working with agents, link: /guide/delegate/agents }
  trust:
    title: Runs on your machine. Your data stays yours.
    items:
      - { title: Runs locally, text: The server listens only on 127.0.0.1. }
      - { title: Plain Markdown, text: Tasks are TASK.md and epics are EPIC.md. You can edit them directly. }
      - { title: Open source (MIT), text: The source code is on GitHub. }
      - { title: No account, text: No sign-up and no hosted service. }
  start:
    eyebrow: Get started
    title: Three steps to an open board.
    lead: You need macOS 15 or later, Homebrew, and Claude Code or Codex.
    steps:
      - { title: Install, commands: [brew install fooqoo/tap/agpj] }
      - { title: Set up your agent and a project, commands: [agpj setup, 'agpj project create "My project"'] }
      - { title: Open the board, commands: [agpj serve] }
    note: "Some features, such as Docs and resident agents, are in public preview. See "
    noteLinks:
      - { text: Feature stages, link: /guide/feature-stages }
    noteJoin: " and "
    noteEnd: .
    primary: { text: Read Install and start, link: /guide/getting-started }
    secondary: { text: GitHub, link: https://github.com/FooQoo/agent-project }
---

<LandingPage />
