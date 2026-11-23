# ドキュメントと画像

ドキュメントサイトは`docs/`にあります。このページでは、プレビューとビルドの方法、スクリーンショットとデモ動画の再生成方法を説明します。

## ドキュメントサイト

ドキュメントサイトは`docs/`にあり、VitePressでビルドします。利用者向けのガイド（`docs/guide/`）と、この開発者向けの章（`docs/dev/`）があります。英語が正本で、英語を変えるたびに`docs/ja/`の日本語ページも揃えます。`main`から`.github/workflows/docs.yml`でGitHub Pagesに公開し、`docs/**`、`web/**`、デモのスイートを変えるプルリクエストではビルドとテストだけを行います。同じワークフローが、サイトのあとに[デモ](./getting-started/setup.md#demo-build)をビルドし、`web/dist-demo/`を`docs/.vitepress/dist/demo/`へコピーするので、デモは`/<リポジトリ名>/demo/`で配信されます。続けてその出力に対してデモのスイートを実行し、失敗したときはアップロードの前に止まります。詳しくは[ブラウザのデモ](./concepts/demo.md#手元で確かめる)を参照してください。デモはVitePressのページではないため、ガイドからのリンクには完全なURLを使います。

```sh
pnpm --dir docs install
pnpm --dir docs dev --port <port>       # ライブプレビュー
pnpm --dir docs build                   # リンク切れも検査
pnpm --dir docs preview --port <port>   # ビルドしたサイトを配信
```

`docs/`の下を変更したら、コミット前に`pnpm --dir docs build`を実行してください。`docs/`の外にあるリポジトリのファイルへは、GitHubの絶対URLでリンクします。

トップページにはMarkdownの本文がありません。文言は`docs/index.md`と`docs/ja/index.md`のfrontmatterにある`landing`に置き、`docs/.vitepress/theme/LandingPage.vue`がそれを配置します。言語に合ったスクリーンショットを選び、ダークテーマではカンバンの画像をダーク版に切り替えるのもこのコンポーネントです。文言は両方のファイルで直し、キーをそろえてください。frontmatterのリンクはbuildのリンク切れ検査の対象外なので、サイトのテストがトップページのすべてのリンクを開いて確かめます。配色は`custom.css`の`--lp-*`変数で、アプリの列と待機中の色に合わせています。

サイトとデモはリポジトリ名のパスで配信するので、ベースパスもリポジトリ名に従います。`docs/.vitepress/repo.ts`と`web/demo-build.ts`が`GITHUB_REPOSITORY`（`owner/name`の形。GitHub Actionsが設定します）を読み、手元のビルドでは`FooQoo/agent-project`を既定にします。サイトのテストも`e2e/site-base.ts`で同じように読みます。以下の例は既定の名前を使います。

英語のトップページ(`/agent-project/`)は、ブラウザの中で表示する言語を選びます。

- ブラウザの優先言語が日本語(`navigator.languages`の先頭が`ja`で始まる)なら、`/agent-project/ja/`へ移ります。
- ナビの言語切り替え(PCでもスマートフォンでも)で選んだ言語は、`localStorage`の`agent-project-docs-lang`に保存します。以後はブラウザの言語より保存した選択を優先します。
- 移動するのは英語のトップページだけです。個別ページへの直接のリンク、日本語のページ、サイト内の移動では移りません。ストレージが使えないときは選択を保存せず、ブラウザの言語で決めます。

移動はブラウザの中だけで行うため、どのページもビルドしたHTMLは検索エンジン向けに変わりません。仕組みは`docs/.vitepress/theme/languageChoice.ts`にまとめています。`config.ts`がトップページにだけ、charsetの直後、スタイルシートより前に移動用のスクリプトを埋め込むので、英語のページはほとんど見えないうちに移ります。スクリプトは設定の`base`を使います。選択の保存は、テーマの`enhanceApp`が言語切り替えのリンクのクリックを受けて行います。`e2e/docs/language.spec.ts`のサイトテストが、日本語と英語のブラウザ、保存した選択、スマートフォンのメニュー、ストレージが使えない場合、直接のリンクを確かめます。

図は一つの流れに絞って描きます。フローチャートとシーケンス図は文字サイズを保ち、本文より横に広い場合は枠内でスクロールします。システム構成図は左から右の流れに揃え、大きな図は目的ごとに分けてください。サイトのテストでは、図の文字サイズと、本文に未処理の太字記号が残っていないことを確認します。表は読みやすい最小列幅を保ち、本文より広い場合は記事の中で横にスクロールします。セル内の文章は折り返せます。

ガイドの画像とデモ動画は、次のスクリプトで再生成します。どちらも先に[Web画面とCLIをビルド](./getting-started/setup.md#ビルド)し、[検証](./getting-started/pull-request.md#検証)のPlaywrightのブラウザをインストールしておく必要があります。

```sh
node e2e/scripts/docs-screenshots.mjs   # docs/images/guide/*.png(英日26枚)
node e2e/scripts/demo-video.mjs         # docs/public/videos/(ffmpegも必要)
```

どちらのスクリプトも、一時的な`AGPJ_HOME`と専用のループバックポートを使うため、普段のカンバンやポート7420には影響しません。生成した画像と動画は、コミット前に確認してください。

### 撮影を決まった結果にする

同じコミットから2回撮影すると同じ画像になります。そのため、画像が変わったときは製品が変わったと分かります。3つのスクリプト(`readme-screenshots.mjs`、`docs-screenshots.mjs`、`demo-video.mjs`)は、`e2e/scripts/lib/sample-board.mjs`の設定を共有します。

- **「現在」を1つに固定します。** `FIXED_NOW`(`2026-10-15T09:41:00Z`)を、`AGPJ_FIXED_NOW`としてCLIとサーバーに渡します。製品のコードは時刻を`crate::clock::now()`から取り、この変数にUTCの時刻が入っていればその時刻を返すので、サンプルデータに書かれる時刻はすべて同じになります。カードの更新日のようにタスクファイルの更新時刻から読む日付も、これに従います。変数がない、または読み取れない場合は何も起きません。
- **ブラウザにも同じ時計を持たせます。** `freezeClock(context)`が`context.clock.setFixedTime`を呼ぶので、どのページでも`Date.now()`と`new Date()`が同じ時刻を返します。「同期済み」の時刻とカードの日付が、データと一致します。タイマーは動き続けます。コンテキストのタイムゾーンもUTCです。
- **描画を安定させます。** Chromiumは`--disable-gpu --disable-lcd-text --font-render-hinting=none`で起動します。`settle()`は繰り返しのアニメーション(スピナー)を最初のフレームで止め、そのあと3回続けて同じ画面が撮れるまで待つので、フェードやカルーセルの途中は撮りません。

この手順で撮ったスクリーンショットは、前回と画素まで一致します。ただし、スマホとタブレットの一部の画像では、角丸の枠線のアンチエイリアスにより、数画素がチャンネルあたり1〜2段階ずれることがあります。録画した動画は実時間で進むため、実行のたびにフレームが少しずれ、許容を付けて比べます。

### 撮影結果をコミット済みのファイルと比べる

`e2e/scripts/compare-captures.mjs`は、撮影したファイルのうち、見た目がリポジトリのコミット済みファイルと異なるものだけを報告します。`AGPJ_MEDIA_OUT`に一時フォルダーを指定して撮影し、比べます。

```sh
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/readme-screenshots.mjs
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/docs-screenshots.mjs
AGPJ_MEDIA_OUT=/tmp/captures node e2e/scripts/demo-video.mjs
node e2e/scripts/compare-captures.mjs --captured /tmp/captures            # 変わったファイルを一覧する
node e2e/scripts/compare-captures.mjs --captured /tmp/captures --json     # {"changed":[{"file","status","target"}],"unchanged":N}
node e2e/scripts/compare-captures.mjs --captured /tmp/captures --apply    # 変わったファイルを所定の場所へコピーする
```

`readme/`は`docs/images/`、`guide/`は`docs/images/guide/`、`videos/`は`docs/public/videos/`と比べます。終了コードは、差がなければ0、あれば1、使い方の誤り・読めないファイル・PNGもMP4もないフォルダーなら2です。新しいファイルは変更(`added`)として数えます。`--apply`は、変わった動画の`.jpg`ポスターもコピーします。

- PNGはRGBAに復号し、画素で比べるため、エンコーダーのメタデータや圧縮の違いは差になりません。`--image-tolerance <n>`を付けると、チャンネルあたり`n`までのずれを許します。縁のアンチエイリアスによるノイズを無視したいときに使います(例: `4`)。
- MP4は`ffmpeg`で1秒に1フレーム、480×300に復号します。フレームがすべて同じなら変更なしです。そうでなければ、各フレームを前後1秒のフレームと比べ、画素の0.3%を超えて違うフレームが全体の1割を超えたときだけ変更とします。カードの日付のような小さな変化は動画では気づけません。その場合はスクリーンショットに現れます。
- `--against <dir>`を付けると、リポジトリではなく別の`AGPJ_MEDIA_OUT`フォルダーと比べます。2回の撮影が一致するかを確かめるときに使います。

### mainでの自動撮り直し

`.github/workflows/ci.yml`の`retake-media`ジョブが、手作業で撮り直さなくてもコミット済みの画像と動画を最新に保ちます。プルリクエストで変更を確認するときは、これまでどおり上のスクリプトを自分で実行します。このジョブが追うのは`main`だけです。

- **動くとき。** `main`へのpushで、Rust・web・カバレッジ・E2Eの各ジョブがすべて成功した後にだけ動きます。E2Eビルドのジョブがアップロードした CLI のバイナリを使うので、ビルドし直しません。プルリクエストでは動きません。
- **やること。** 3つの撮影スクリプトを一時的な`AGPJ_MEDIA_OUT`フォルダーに実行し、`compare-captures.mjs --image-tolerance 4`で比べます。ファイルのバイト列ではなく復号した画素を比べます。見た目の差がなければ、何も push せずに終わります。
- **パスの検査。** 差があれば`--apply`で変わったファイルを所定の場所に写します。そのうえで、変更または追加されたファイルをすべて列挙し、`docs/images/`と`docs/public/videos/`の外のパスが1つでもあれば、push せずにジョブを失敗させます。
- **push。** `agpj-images`GitHub App のインストールトークン(`actions/create-github-app-token`、権限は Contents: write のみ、シークレットは`AUTO_IMAGES_APP_ID`と`AUTO_IMAGES_APP_PRIVATE_KEY`)を取得し、App の名義でコミットして`main`へ push します。`main`のルールセットはこの App にプルリクエストとステータスチェックの規則を免除しているため、[AGENTS.md](https://github.com/FooQoo/agent-project/blob/main/AGENTS.md)の「`main`へ直接 push しない」の唯一の例外です。push の前に`main`を取得し、撮影したコミットより`main`が進んでいれば、push せずに止めて、新しいコミットの実行に撮り直しを任せます。push が拒否されても、失敗ではなく警告として報告します。
- **競合しない。** ジョブは専用の並行実行グループ`retake-media`を持ち、実行中のものを取り消さないので、2つの撮り直しが同時には動きません。
- **ループしない。** App トークンでの push は CI を再び起動します。実行者が`agpj-images[bot]`のときはジョブを飛ばします。それがなくても、撮影は決まった結果になる(上記)ので、ボット自身のコミットの撮り直しには見た目の差が出ず、何も push しません。
- **サイトの公開。** ワークフローのトークンではなく App トークンで push するので、`.github/workflows/docs.yml`が起動します。その`push`の対象は`docs/**`を含むため、新しい画像でサイトとデモが再ビルドされます。

ジョブは`ubuntu-latest`で撮影するため、フォントや描画が Mac とは異なります。このジョブを入れた後の最初の実行では多くのファイルが置き換わることがあります。以降は、製品が変わったときだけ差が出ます。

## ページを移してもURLを保つ

ガイドの古いURLは、READMEやプラグインのREADME、他の人のリンクに載っています。そのため、移したページは古いアドレスでも応答し続ける必要があります。`docs/.vitepress/redirects.ts`は、移したページの一覧を、古いパスと新しいパスの組で持ちます。パスは先頭のスラッシュも拡張子もない形で、たとえば`guide/use/board`と`guide/watch/board`です。ビルドは組ごとに、古いパスに小さなHTMLページを書き出し、新しいページへ転送します。

- ページを移す、または名前を変えるときは、一覧に1件を足します。ファイルが英語と日本語（`ja/...`）のパスの両方に展開します。日本語のページも同じ変更で移してください。
- 転送先のページがビルドされていないと、ビルドは失敗するので、一覧の書き間違いは公開前に分かります。古いパスがまだ実際のページのままでも失敗するので、エントリを足すときは古いファイルを削除してください。
- 転送ページは検索の索引を作ったあとに書くので、検索には出ません。

## 公開済みのリリースノートを検査する

サイトのヘッダーにはバージョンを表示しません。ビルドの前に、`node scripts/release/docs-release.mjs`が公開済みのGitHub Releaseを読んでリリースノートを検査し、形式が崩れたReleaseがあればビルドを止めます。

リリースノートは2か所にあります。

- GitHub Releaseの本文（英語）。`releases/vX.Y.Z.md`がそのレビュー用の原稿です。
- サイトのリリースノートのページ。`docs/releases.md`とその日本語訳の`docs/ja/releases.md`で、公開済みのバージョンを新しい順に並べます。Releaseを公開したら、新しいバージョンを両方のページに書き足してください。スクリプトはこのページを書きません。

```sh
node scripts/release/docs-release.mjs --github --repo FooQoo/agent-project   # 公開するサイト
node scripts/release/docs-release.mjs --fixture                              # 公開済みの状態を手元で確認
node scripts/release/docs-release.mjs --none                                 # リリース前の状態（既定）
pnpm --dir docs build
```

- 対象にするのは、`draft=false`、`prerelease=false`で、タグが`vMAJOR.MINOR.PATCH`の形のリリースだけで、その中でバージョンが最も新しいものを使います。`Cargo.toml`のバージョンは使いません。リリースノートは英語だけで書きます。各リリースの本文には、`## Changes`、`## Fixes`、`## Breaking changes`、`## Required update steps`の4つの見出しが1回ずつ要ります。見出しが欠けていたり、重なっていたりすると、スクリプトは止まります。書くことがない見出しも残し、その下に`None.`と書きます。
- 見つけた状態を`docs/.vitepress/release.json`（Gitは無視します）に書き出します。`--fixture`は`releases/`のレビュー用の原稿を使い、fixtureの状態として記録します。
- Pagesのワークフローは、`main`へのプッシュ、`release`イベント（published、edited、deleted）、手動の実行で、ビルドの前にこのスクリプトを`--github`で実行します。APIの呼び出しが失敗する、またはリリースが不正なときは、ビルドが止まり、公開済みのサイトはそのまま残ります。プルリクエストは、コミットしてある状態で、オフラインでビルドします。ワークフロー自身のトークンで公開したリリースは、ワークフローを起動しないことがあります。その場合は手動で実行してください。
- `releases/vX.Y.Z.md`は、リリースノートのレビュー用の原稿です。公開後は、GitHub Releaseの本文が正本です。
- Releaseを公開する前に本文を確かめるには、GitHub APIの一覧と同じ形のファイルを`--releases-json`に渡してスクリプトを実行します。

## UIのStoryとカバレッジ

Storybookでは、部品の状態を個別に表示したり、アプリと同じUI部品を組み合わせてページ全体を表示したりできます。各部品のStory、焦点を絞ったテスト、専用スタイル、カバレッジ台帳はソースの近くに置きます。部品やページを変更したら、Storyと画面状態の記録も更新し、対象の言語・テーマ・表示幅で画面を確認してください。自動検査の現在の範囲は[開発環境のセットアップ](./getting-started/setup.md#storybook)を参照してください。

## READMEの画像

[Web画面とCLIをビルド](./getting-started/setup.md#ビルド)したあと、READMEの画像を再生成します。

```sh
pnpm --dir e2e install --frozen-lockfile
pnpm --dir e2e exec playwright install chromium
node e2e/scripts/readme-screenshots.mjs
```

スクリプトは一時的な`AGPJ_HOME`に使い捨てのプロジェクトを作り、空いているループバックのポートで独自のサーバーを起動し、PCのカンバン、エピックの進捗パネル、タブレット幅（1024×768）で開いたカンバン（リモートの入口の背後）、スマホの4画面（タブ付きの進行中レーン、スライドして移動とアーカイブのボタンを出した選択中のカード、選択肢を選んだ質問、下部のドロワー）を英語と日本語で撮影します。普段のカンバンやポート7420は使いません。ブラウザはビルドした画面を実際のAPIに接続して描画するので、画像はモックではありません。詳細の4つのタブ、タスクの編集画面の折りたたんだ完了条件の欄、ブラウザのエラーを確かめ、終わったら一時データを消します。

`docs/images/`の14枚は、コミット前にすべて確認し、サンプルの文章に個人情報を含めないでください。スクリーンショットは回帰テストやスマホのキーボードの確認の代わりにはなりません。`docs/images/guide/`のガイドの画像（26枚）は上の別のスクリプトで作るので、作り直したときはそれも確認してください。
