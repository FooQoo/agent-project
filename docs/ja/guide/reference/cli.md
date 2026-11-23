---
title: CLIリファレンス
---

# CLIリファレンス

ブラウザは作業の確認と編集に、`agpj`はエージェントの更新とプロジェクトの管理に使います。ほとんどのコマンドは、`--project NAME`を明示します。例は`My project`を使っているので、自分のプロジェクト名に置き換え、CLIが返すタスクIDを使ってください。各オプションは`agpj --help`か`agpj <command> --help`で確認できます。

## コマンドの一覧

| したいこと | コマンド |
| --- | --- |
| タスクを追加する | `add` |
| タスクやプロジェクトの情報を読む | `show`、`ls`、`snapshot` |
| タスクを移す、エージェントの状態を報告する | `mv`、`state` |
| 後で効く判断を記録する | `history add` |
| 組手・スリープ・壁打ちを切り替える | `mode` |
| スリープ中にサブエージェントが下した判断を記録する | `decision add` |
| タスクの本文を編集する | `edit` |
| タスクをリリースに割り当てる | `version`、`edit --planned-version`、`epic plan` |
| 画像を添付する | `attachment` |
| 成果を依頼者に届ける | `result add`、`result ls` |
| タスクをまとめて順序づける | `epic`、`progress` |
| 変更を待つ | `wait` |
| プロジェクトとフォルダを管理する | `project`、`workspace` |
| カンバンをブラウザーで開く | `open` |
| スマホ用のアドレスと合言葉を確認する、リモートアクセスの設定を見る | `show remote` |
| リモートアクセスの設定（ngrokかTailscale、ドメイン、トークン）を保存する | `setup remote` |
| サーバーとHookを動かす | `serve`、`stop`、`hook` |
| Agent Projectプラグインを導入・更新する | `setup`、`update` |
| 削除する（人だけ） | `rm`、`project rm`、`epic rm`、`init` |

## タスクを追加して確認する

タスクを追加して、カンバンを確認します。

```sh
agpj --project "My project" add "Write the setup guide" --assignee ai --actor ai
agpj --project "My project" ls --json
agpj --project "My project" show 1 --json
agpj --project "My project" snapshot --json
```

`--assignee`は担当を指します。`--actor`は変更を行う主体を指し、権限を決めます。人には`human`、エージェントには`ai`を使います。`snapshot`は、エピックとファイルの問題を含む、プロジェクトの全体像と履歴を返します。

## 作業を報告する

作業の開始を報告します。

```sh
agpj --project "My project" mv 1 doing --actor ai --reason "Start implementation"
agpj --project "My project" state 1 running --actor ai --reason "Writing and checking the guide"
```

列の移動や状態の変更では、`--reason`で理由を指定できます。指定は任意で、上限は10,000 Unicode文字です。理由を省略するか空白にすると、現在の状態から生成します。これらの操作は履歴を自動で追加するので、同じ操作の履歴を重複して追加しないでください。質問は5,000文字まで、回答のメモは2,000文字までです。

列や状態を変えないが、後で効く判断（カードをTODOに据え置くなど）を記録します。

```sh
agpj --project "My project" history add 1 --reason "公開文面は実機確認の結果で変わるため、確認が終わるまでTODOに置く"
agpj --project "My project" history add 1 --reason-file progress.txt
```

コマンドが時刻を付け、AIの書き込み権限を適用します。actor・イベントID・種別・時刻の指定は要りません。根拠・判断・分かっていれば再検討の条件を書きます。一覧を見ただけでは追記せず、`result add`は自動で記録されます。理由を省略するか空白にすると、状態に合った既定の文を生成します。AIは、下書き・アーカイブ済み・完了のタスクに履歴を追加できません。

テキストを受け取るオプションは、次の語が`-`で始まらない場合にだけ、それを値として受け取ります。`--reason --json`のように値を書き忘れると、次のフラグが本文として記録されるのではなく、`invalid_argument`のエラーになります。Markdownの箇条書きのように`-`で始まるテキストを渡すには、`=`でオプションにつなげる（`--reason="- Checked the links"`）か、ファイル（`--reason-file`、`--text-file`、`--desc-file`、`--document-file`）または標準入力（`--desc -`、`--questions-json -`）で渡します。標準入力を使えるオプションでは、単独の`-`はこれまでどおり標準入力を表します。`TASK.md`や`EPIC.md`の全文は`---`で始まるので、`--desc-file`・`--document-file`か標準入力で渡します。対象のオプションは`--reason`、`--text`、`--desc`、`--document`、`--questions-json`、`--title`、`--summary`、`--link`です。

### 成果を届ける {#deliver-a-result}

依頼者が見られる最初の成果ができたら、作業の途中でも記録し、そのまま作業を続けます。成果は短い文章（`--summary`）と、任意の見出し（`--title`。1行で1〜80文字）で、画像（`--file`。複数の画像は繰り返して指定し、指定した順に最大20枚）と、主な成果物へのリンクまたは保存場所（`--link`）を添えられます。`--final`を付けない成果は途中のもの、`--final`を付けると完成したものです。

AIタスクの条件がすべて満たされているのに最終の成果がないとき、`agpj progress`とエージェントのHookが`finalResultMissing`にそのタスクを載せ、エージェントに成果の記録を促します。自動Doneを止めたり遅らせたりはせず、Doneになったタスクにも最初の最終の成果を記録できます。

```sh
agpj --project "My project" result add 1 --title "First draft" --summary "The first 30 seconds are ready" --link ./out/draft.mp4 --actor ai
agpj --project "My project" result add 1 --summary "Top page capture" --file top.png --actor ai
agpj --project "My project" result add 1 --summary "All screens of the mock" --file 01.png --file 02.png --file 03.png --actor ai
agpj --project "My project" result add 1 --summary "Finished and checked" --final --actor ai
agpj --project "My project" result add 1 --published v1.0.0 --link https://example.com/releases/v1.0.0 --summary "Released" --actor ai
agpj --project "My project" result ls 1
```

成果には、タスクごとに次のバージョン番号（1、2、……）が付きます。記録しても列と稼働状態は変わらず、履歴が1件増えます。同じ成果を繰り返し送っても安全です。すでに記録した成果を見つけて`created: false`を返し、何も書きません。`--key`を省くと、内容（種別・要約・リンク・画像のバイト列）で同じ成果かを判断します。`--key <key>`で自分で名前を付けることもでき、別の内容に同じキーを使うと`invalid_value`で失敗します。`--title`を省くと、タイトルは要約の最初の1文を80文字までに切ったものになります（切ったときは最後が`…`）。タイトルは同じ成果かどうかの判断にも含まれるため、別のタイトルで送り直すと、同じ`--key`を使わない限り別の成果になります（同じ`--key`では`invalid_value`で失敗します）。要約とリンクは1〜2,000文字、1つのタスクに残せる成果は100件までです。AIは、人の下書き・アーカイブ済み・完了のタスクには成果を記録できません。ただし、すでに記録した成果の再送は成功します。`--published バージョン名`は公開の記録です。`--link`が必須で、`--final`とは併用できず、バージョン名は`version add`と同じ規則で検証され、要約の先頭に`Published: バージョン名`の行が付きます。Doneのタスクが受け付ける唯一の成果で、最終成果がすでにあっても、AIでも人でも追加できます。

### タスクを終える

終えるには、`TASK.md`に検証を記録し、検証した完了条件の`done`を`true`にします。サーバーが対象のAI担当タスクを自動で完了に移すので（サーバーはHookや待つコマンドが必要なときに起動します）、`mv ... done --actor ai`は実行しません。規則は[Markdownファイルの形式](./file-format)を参照してください。完了したタスクはエージェントから読み取り専用で、作業が必要になったら、ユーザーが再開できます。

### 質問する

ユーザーに質問するには`message ask`を使います。1回で最大20件の質問を出せます。質問は1件ずつ`Q-12`のようなIDを持つメッセージで、プロジェクトに属し、`--task <id>`を付けるとそのタスクが質問を参照して、回答されるまで回答待になります。AI担当で進行中の稼働中のタスクは、`--task`（と`message link`）で同じ保存の中で稼働待になり、履歴に「Q-1 の回答を待ちます。」（英語のプロジェクトでは"Waiting for the answers to Q-1."）が追記されます。エージェントが回答を受け取ると（`message accept`、または回答済みの結果を返す`message ask`・`message wait`）、タスクは稼働中に戻り、履歴に「Q-1 の回答を受け取ったので再開しました。」が追記されます。再開するのは、タスクが参照する質問がすべて回答済みで、AI担当の進行中の稼働待のタスクであるときだけです。別の参照先の質問が未回答のまま残っていれば、1件の回答を受け取っても再開しません。コマンドは1行に1つのイベントを出力し（`--json`ならJSON Lines）、未回答の質問がなくなるまで待ちます。既定の待機時間は600秒です。

```sh
agpj --project "My project" message ask --actor ai --task 1 --questions-json - --json < questions.json
```

`answered`（終了コード0）、`timeout`（終了コード5。質問は未回答のまま残る）、`withdrawn`（終了コード6）のいずれかで終わります。待ち直すには`message wait Q-12 Q-13`を使います。待機にはサーバーが必要です。動いていなければ、質問を保存する前にコマンドが裏で起動します。`--no-wait`を付けると、質問を登録してすぐに戻り、サーバーも要りません。回答の記録、メッセージの確認、質問の取り下げ、タスクからの参照の変更には、それぞれ`message answer`、`message ls`と`message show`、`message withdraw`、`message link`と`message unlink`を使います。人からエージェントへの依頼には`message send`と`message accept`を使います。質問には`images`で画像ファイルのパスを並べて添えられ、コマンドが質問と一緒に保存します。質問の形式と各コマンドは[判断依頼](../delegate/decisions)を参照してください。`progress`は、おすすめのタスクを最大3件`nextTasks`として返します（[エピックと進行表](../watch/epics#進行表を読む)を参照）。エピックと進行表は[エピックと進行表](../watch/epics)を参照してください。

## スリープモード {#sleep-mode}

`mode`はプロジェクトのモードを表示・切り替えます。既定は組手（`sparring`）で、エージェントは待ち受けて、ボードであなたに質問します。スリープ（`sleep`）にすると質問が止まります。

```sh
agpj --project "My project" mode            # 現在のモードを表示
agpj --project "My project" mode sleep      # あなたへの質問を止める
agpj --project "My project" mode sparring   # 既定に戻す
```

毎日の時間帯を決めておくと、自動でスリープになります。時刻はこのコンピューターの時計の`HH:MM`で、終了が開始より前なら日をまたぎます。

```sh
agpj --project "My project" mode schedule --start 23:00 --end 08:00   # 時間帯を決めてオンにする
agpj --project "My project" mode schedule --off                        # オフにする（時刻は残る）
agpj --project "My project" mode auto                                  # 手動のモードを忘れ、時間帯に決めさせる
```

手動のモードは、設定されているあいだ優先されます。時間帯のあいだに設定したものはその時間帯が終わるまで続き、終わるとスケジュールがまた決めます。時間帯の外で設定したものは、次の時間帯が始まるまで続きます（時間帯がなければ変えるまで）。`mode`は、いま効いているモード、その出どころ（`manual`か`schedule`。モードが終わる時刻の`until`も。時間帯の終わり、手動のモードならその保持の終わり）、時間帯を示します。`message ask`と`agpj progress`も同じ決め方を使い、進行表は`mode`の隣に`modeSource`と`modeUntil`を加えます。


### 壁打ちモード {#brainstorm-mode}

壁打ちモードでは、1つのエピックか1つのタスクについて、エージェントがあなたに質問します。対象（タスク番号かエピックID）を付けて始め、組手に戻して終わります。

```sh
agpj --project "My project" mode brainstorm --target INTERACTION-MODES   # エピック
agpj --project "My project" mode brainstorm --target 12                  # タスク
agpj --project "My project" mode sparring                                # 終わる
```

`--target`は必須で、`brainstorm`のときだけ使えます。存在しない対象と、アーカイブ済みのタスクは拒否されます。壁打ちを始めると、エージェントへ「Start brainstorming on ...」という依頼も送るので、`wait`で待ち受けているエージェントはすぐに始めます。進行表には`mode: "brainstorm"`と`modeTarget`が入り、`mode`コマンドも対象を表示します。

エージェントは`--brainstorm`を付けて質問します。タスクには紐付かないので、対象のタスクは動き続けます。

```sh
agpj --project "My project" message ask --actor ai --brainstorm --questions-json '{"questions":[{"body":"Who reads this page first?"}]}'
```

モードが壁打ちでなくなると、`message ask --brainstorm`は`brainstorm_not_active`で拒否されます。`--brainstorm`のない質問は通常の質問で、そのまま受け付けます。スリープが常に優先されます。スリープの時間帯が始まるとモードはスリープになり、すべての`message ask`が`sleep_mode`で拒否されます。

スリープ中の`message ask`は`sleep_mode`で拒否され、ボード・通知・チャットのどこにも届きません。エージェントはサブエージェントに判断させ、その結論で進め、タスクに記録します。

```sh
agpj --project "My project" decision add 1 \
  --question "古い行を削除するか残すか" --choice "残して移行する" \
  --reason "削除は元に戻せないため" --decided-by subagent:reviewer \
  --basis "src/file_store/ops/task.rs" --basis "Q-12"
```

`--decided-by`は`subagent:NAME`の形で指定し、`--basis`は必須です。判断の根拠にしたコードのパス、ドキュメント、過去の質問（`Q-<n>`）をそれぞれ書きます。履歴に1件、`スリープモード中にsubagent:reviewerが判断しました。質問: ... 選んだ案: ... 理由: ... 根拠: ...`が追加され、カードの履歴で、何を・誰が・なぜ・何を根拠に決めたかを確認できます。モードはプロジェクトごとに保存され、`agpj progress --json`の`mode`に出ます。

## ファイルを使って本文を編集する

長いMarkdown本文は、ファイルに書いてから読み込ませると編集しやすくなります。

```sh
agpj --project "My project" edit 1 --desc-file task.md
```

説明は50,000文字まで、エピックの詳細（`EPIC.md`）は100,000文字まで使えます。ファイルは[Markdownファイルの形式](./file-format)に従う必要があります。標準入力から説明を読むには、`--desc -`を使います。

`edit --desc-file`は、本文と完了条件を置き換え、`TASK.md`の最新の履歴と質問への参照（`questions`）を保ちます。入力ファイルの`history`と`questions`は無視されるため、未回答の質問を参照しているタスクは回答待のままです。参照を変えるには`message link`・`message unlink`を使います。CLIや画面からの移動・状態の変更などの操作は、`TASK.md`に履歴を追加し、`show`・`snapshot`・ブラウザは同じ履歴を読みます。

## エピックの詳細を編集する

エピックの詳細は、frontmatterを含む`EPIC.md`の全文です。文字列で直接渡すか、ファイルや標準入力から読み込みます。

```sh
agpj --project "My project" epic edit LAUNCH --document-file epic.md
agpj --project "My project" epic edit LAUNCH --document -
agpj --project "My project" epic edit LAUNCH --waves 3 1,2
```

`--document`と`--document-file`は同時に指定できません。どちらも100,000文字まで受け付けます。

`--waves`は、エピックのタスクの順序を設定します。ウェーブごとに1つの値を渡し、値はタスクID、またはカンマでつないだ複数のタスクIDです。同じウェーブのタスクは並行して進められ、ウェーブは、その前のウェーブのタスクがすべてDoneになってから始まります。上の例では、タスク3が最初のウェーブ、タスク1と2が次のウェーブです。詳しくは[作業の順序を決める](../watch/epics#作業の順序を決める)を参照してください。

エージェントの`add`は、通常の未着手のタスクを作ります。計画だけを頼まれたときは、`--backlog`を付けて、カードを代わりにバックログに置きます。`epic add`にレーンはありません。エージェントのエピックは通常のエピックで、計画だけのエピックは、カードを`--backlog --epic <ID>`で作ったものです。

```sh
agpj --project "My project" add --title "Plan the launch" --assignee ai --actor ai --backlog --epic LAUNCH
```

バックログは、まだ始めてはいけない確定済みの計画を置く場所です。エージェントは始めず、バックログのカードへのエージェントの`mv`と`state`は`task_in_backlog`（終了コード1）で失敗します。カードは人が未着手の末尾へ送ります。エージェントが送るのは、ユーザーに指示されたときだけで、`--reason`にそのことを書きます。コマンドラインでは、カードを`todo`へ移して送ります。`send`という別のコマンドはありません。

```sh
agpj --project "My project" mv 12 todo --actor human
agpj --project "My project" epic send LAUNCH --actor human
agpj --project "My project" epic send LAUNCH --actor human --tasks 1,2
```

`mv <id> todo`は1枚のカードを送ります。バックログのカードをDoingやDoneへ移すのは人の操作で、エージェントは`task_in_backlog`になります。`epic send`は、エピック全体、または`--tasks`で指定したカードだけを、全部移すか何も移さないかの一回の保存で送ります。エージェントはどちらも、ユーザーに指示されたときだけ実行でき、`--force`で送信を確認することはできません（`human_only`）。DoingのタスクをTodoに戻す（DoingのタスクへのAIの`mv <id> todo`）ことは人だけの操作で、エージェントが実行すると`human_only`になります。エージェントは自分の稼働中のタスクを`state <id> paused --actor ai --reason "<理由>"`で一時停止できますが、止まりを指摘するリマインダーを受け、作業していないときにだけ使います。エージェントはタスクをDoneに移しません。対象のタスクは自動でDoneに移ります（「タスクを終える」を参照）。すでにバックログを出たカードは、ほかのカードと同じように移ります。ブラウザで人が編集中のカード（`draft: human`）は送れません（`task_is_draft`）。履歴には「人が Todo へ送りました。」または「エージェントが Todo へ送りました。」と残り、エージェントの場合は続けて`--reason`の内容が入ります。

計画に未回答の質問があると、送信は`send_with_open_questions`（終了コード1）で断られ、メッセージに質問のIDが並びます。`mv`（または`epic send`）に`--force`を付けると、そのまま送ります。質問は未回答のまま残り、回答があるまでカードは回答待です。

```sh
agpj --project "My project" mv 12 todo --actor human --force
```

人は、未着手の未着手のカード（Idleで成果なし）をバックログへ戻せます。エージェントが行うと`human_only`です。

```sh
agpj --project "My project" mv 12 backlog --actor human
```

人がブラウザでエピックを下書きにしている間は、`--waves`を含む`epic edit`が`epic_is_draft`（終了コード1）で失敗し、何も変わりません。保存か破棄が済むまで待つか、残った下書きなら確定を頼んでください。`epic attachment`とタスクの所属の変更は、引き続き使えます。

## 一覧を絞り込む

```sh
agpj --project "My project" ls --column doing --json
agpj --project "My project" ls --column backlog --json
agpj --project "My project" ls --awaiting-answer --json
agpj ls --all-projects --json
```

`--state`には`idle`・`running`・`paused`を指定でき、`--awaiting-answer`は未回答の質問を待つタスクだけを返します。`ls --all-projects`は、全プロジェクトを名前順に読み、タスクの説明と履歴を完全な形で返します。`--column`と`--state`の絞り込みはすべてのプロジェクトに適用されます。`--project`とは組み合わせません。

## タスクをバージョンに割り当てる {#plan-tasks-for-a-version}

プロジェクトはバージョンの一覧を持ち、その1つをリリースバージョンにできます。タスクは1つのバージョンに割り当てるか、どのバージョンにも割り当てません（未定）。バージョンの名前は空白を含まない1〜64文字（例: `v1.8.0`）で、書いたとおりに保存し、比較します。

```sh
agpj --project "My project" version add v1.9.0
agpj --project "My project" version release v1.8.0
agpj --project "My project" version release
agpj --project "My project" version release --clear
agpj --project "My project" version ls --json
```

プロジェクトのバージョンは、その[ドキュメント](../watch/docs)の`docs/release/`直下にある`*.md`ファイルで、新しいものから並びます（`v1.10.0`は`v1.9.0`より前。`TEMPLATE.md`はリリースのテンプレートで、バージョンにはなりません）。`version add VERSION`は`docs/release/TEMPLATE.md`をコピーして`docs/release/VERSION.md`を作り（テンプレートがなければ、標準のリリースノートのテンプレートを先に書きます）、既存のファイルは上書きしません。すでにファイルがあるバージョンは、一覧にあるものとして扱います。あわせて、そのリリース作業のカードを、担当がAIでBacklogに1枚作ります。タイトルは`Release VERSION`（表示言語が日本語なら`VERSION をリリースする`）で、予定のバージョンはそのバージョンです。やることには、テンプレートをもとに`docs/release/VERSION.md`のリリースノートを書くことと、リリース作業そのもの（リリースを公開し、利用者に届いたことを確かめる）が並びます。人がTodoに送ってから着手し、同じバージョンをもう一度追加しても2枚目は作りません。このカードがどの経路でもDoneになり、VERSIONがリリースバージョンのときは、リリースバージョンが登録済みの版のうちいちばん近い新しい版へ進み、カードの履歴に残ります（`Release version moved from v2.1.0 to v2.2.0.`、日本語では`リリース版を v2.1.0 から v2.2.0 に進めました。`）。より新しい版がなければリリースバージョンは変わらず、そのことも履歴に残ります。リリースバージョンが別の版のとき、Done以外のレーンへ動かすとき、すでにDoneのカードのときは何も変わりません。バージョンを追加してもリリースバージョンは変わりません。ファイルを自分で作ってもバージョンを追加できます。`version release VERSION`は、まだ一覧にないバージョンを同じ方法で載せたうえでリリースバージョンにします。引数なしではリリースバージョンを表示します。あとでリリースバージョンのファイルを削除すると、ファイルが戻るまで、プロジェクトはリリースバージョンがないものとして読みます。リリースバージョンを変えてもタスクは動きません。タスクは割り当てられたバージョンの名前を保存しています。

### リリースドキュメントを生成する {#generate-a-release-document}

```sh
agpj --project "My project" release doc --release-version
agpj --project "My project" release doc --version v1.9.0
```

`release doc`は、1つのバージョンのリリースドキュメントをプロジェクトの[ドキュメント](../watch/docs)の`docs/release/VERSION.md`に書きます。アーカイブ済みを含むすべてのカードを読み、そのバージョンについて次のように分けます。

- **予定**：いま予定のバージョンがこのバージョンのカード。
- **検証済み**：予定のカードのうち、受入条件があり、すべて満たしたもの。
- **受入条件なしで完了**：予定のカードのうち、Doneで受入条件がまったくないもの。検証する手段がないため、検証済みにも残作業にも数えません。
- **残作業**：それ以外の予定のカードのうち、まだ検証済みでないもの。
- **別のバージョンへ移した**：このバージョンから外れたカード。後のバージョン、前のバージョン、バージョンなしのいずれへの移動も含みます。履歴にこのバージョンから別のバージョンへの変更があり、各行に移動先を書きます。

各項目にはカード（`#12`）、成果と履歴の件数、各成果の日時とリンクを載せ、原記録は`agpj show 12`でたどれます。最後の**公開**の節は、公開済みのカードとそれ以外を分けます。予定のカードを公開済みと数えるのは、成果の本文に`Published: バージョン名`の行があり、公開物へのリンクが付いている場合だけです。行はあってもリンクがない成果、そうした成果がないカードは、完了していても「確認できない」に載せます。冒頭の集計にも`公開済み(公開記録あり): N / M`が出ます。エージェントの使い方は[バージョン別レポートとテーマの振り返り](../delegate/release-reports)を参照してください。

文書は手で編集して構いません。生成する部分は`<!-- agpj:generated:start -->`と`<!-- agpj:generated:end -->`の行の間にあります。再生成で置き換わるのはその間だけなので、前後の文章は残り、同じプロジェクトデータから2回生成しても何も変わりません。この行がないファイル（`version add`がテンプレートから作ったものなど）は、文章を残したまま末尾にブロックを追加します。マーカー行が欠けている、重複している、順序が逆のファイルは`release_doc_refused`で拒否してそのまま残し、生成中に内容が変わったファイルも同様です。保存は一度に行うので、失敗しても書きかけの文書は残りません。

`--version VERSION`はそのバージョンで生成し、一覧にないバージョンは`version add`と同じようにまず一覧に加えます。`--release-version`はプロジェクトのリリースバージョンを使い、未設定なら`no_release_version`（終了コード1）で失敗して何も書きません。バージョンごとに別のファイルになるので、別のバージョンで生成すると文書が2つ並び、最初の文書には触れません。ファイルはカンバンの「ドキュメント」から開けます。`docs/release/`にバージョンが並びます。

生成しても何も公開されません。書くのはプロジェクトの共有フォルダ内の1ファイルだけで、GitHubなどへは何も送りません。

カードの予定のバージョンは、`TASK.md`のフロントマターにある`plannedVersion`キーなので、手で設定・削除することもできます。

カードは追加するときにも、あとからでも割り当てられます。

```sh
agpj --project "My project" add "Fix the login bug" --assignee ai --actor ai --release-version
agpj --project "My project" edit 15 --planned-version v1.9.0
agpj --project "My project" edit 15 --release-version
agpj --project "My project" edit 15 --unplanned
```

`--planned-version VERSION`は一覧にあるバージョン（`version add`済み、または`docs/release/`にファイルがあるもの）が必要です。`--release-version`はその時点のリリースバージョンの名前を保存し、`--unplanned`は割り当てを外します。3つは同時に指定できません。プロジェクトにリリースバージョンがないときの`--release-version`は`no_release_version`（終了コード1）で失敗し、何も変更しません。実際に変わったときだけカードの履歴に記録し、すでにそのバージョンのカードに同じバージョンを割り当てても何も変わりません。予定のバージョンは、Doneやアーカイブされたカードでもエージェントと人のどちらも変更できる唯一の項目です。Doneのカードのそのほかの編集はエージェントには拒否されたままなので、Doneのカードで`--planned-version`とタイトルや説明の変更を組み合わせた編集は全体が拒否されます。エージェントは人の下書きを変更できません。

エピックのカードをまとめて割り当てるには`epic plan`を使います。

```sh
agpj --project "My project" epic plan LAUNCH --planned-version v1.9.0
agpj --project "My project" epic plan LAUNCH --release-version --only-unplanned
agpj --project "My project" epic plan LAUNCH --unplanned
```

カードは1枚ずつ保存するため、途中で失敗すると先のカードは変更済みのままで、コマンドを再実行すれば残りが済みます。結果は、変更したカード（`changed`）、すでにそのバージョンだったカード（`unchanged`）、飛ばしたカード（`skipped`）の数と番号を返します。アーカイブされたカード、Doneのカード（出荷済み）、人が下書き中のカードは常に飛ばします。`--only-unplanned`を付けると、すでにバージョンが割り当てられたカードも飛ばします。`--unplanned`と`--only-unplanned`は同時に指定できません。

`ls`と`progress`は1つのバージョンに絞り込めます。

```sh
agpj --project "My project" ls --release-version
agpj --project "My project" ls --version v1.8.0 --epic LAUNCH --json
agpj --project "My project" ls --unplanned --column todo
agpj --project "My project" progress --version v1.8.0 --json
```

`--version VERSION`、`--release-version`、`--unplanned`は同時に指定できず、ほかの絞り込みと組み合わせられます。リリースバージョンがないプロジェクトでの`--release-version`は`no_release_version`で失敗します。`ls --all-projects`は`--version`と`--unplanned`を受け付けますが、リリースバージョンはプロジェクトごとに違うため`--release-version`は受け付けません。JSONのタスクには`plannedVersion`が付き（割り当てがなければ省かれます）、テキストでは`[plan VERSION]`と表示します。ルートの`agpj --version`は、これまでどおりプログラムのバージョンを表示します。

バージョンの絞り込みはタスクを隠すだけです。`progress`では、エージェントが着手できるタスクをプロジェクト全体で先に決めるので、前のウェーブのタスクが絞り込みで隠れていても、そのタスクは着手可能になりません。`nextTasks`には、絞り込みに合う着手可能なタスクの先頭3件が入ります。リクエスト、回答、閉じるエピックは絞り込みの対象外です。

## プロジェクト {#projects}

```sh
agpj project create "My project"
agpj project ls --json
agpj --project "My project" project rename "Website launch"
agpj project bind "My project" --folder ./docs
```

名前を変えても、プロジェクトのID・フォルダの登録・タスクは保たれます。フォルダの登録とプロジェクトの削除は、[データとバックアップ](../operations/data#projects-and-folders)に説明があります。

## 変更またはタスクを待つ

`wait`には二つのモードがあります。`--after <cursor>`は[エージェントとの連携](../delegate/agents#変更を待つ)のとおりプロジェクトの変更を待ちます。`--until task`は、対象となるAI担当タスクを一件返します。リッスンを始める前から未着手にあったタスクや、未稼働のまま進行中にあるタスクも対象です。

```sh
agpj --project "My project" wait --until task --idle-timeout 300 --json
```

`--until task`には`--idle-timeout`が必須です。1〜86,400秒を指定し、既定値はありません。一回の呼び出しで返すのは進行表と同じ順序の一件で、タスクを占有したり変更したりしません。取得したタスクを解決したら、再び`wait`を呼び出します。セッション全体の流れと、回答待のタスクの扱いは[エージェントとの連携](../delegate/agents#listen-for-the-next-task)を参照してください。

取得成功時は、一つのJSONオブジェクトを出力して終了コード0を返します。

```json
{"kind":"task","projectId":"<id>","cursor":"<cursor>","task":{"id":1},"taskPath":"…/tasks/1/TASK.md","documentPath":null}
```

タスクより先に、`{"kind":"answers",…}`（エージェントがまだ受け取っていない回答）や`{"kind":"requests",…}`（未処理の依頼）を返すことがあり、回答が先です。それぞれ`message accept`で受け取ってから、あらためて待ちます。

指定した時間に対象タスクがなかった場合は、`{"kind":"idle_timeout","projectId":"<id>","idleTimeoutSeconds":300}`を出力して終了コード0を返します。これは対象なしでの正常終了であり、それまでの作業成功を示しません。`board_unavailable`・`project_not_found`・不正なプロジェクトデータ・引数誤りは、引き続き終了コード0以外のエラーです。`--timeout`は`--after`モードだけに指定でき、変更待ちが成立しなければエラー`wait_timeout`になります。質問への回答にはなりません。

## サーバーを起動する

サーバーを自分で起動することは、ほとんどありません。`wait`、回答を待つ質問、エージェントのHookが、動いていなければ裏で起動します。起動したコマンドは、標準エラーに1行出します。

```text
Started the Agent Project server at http://127.0.0.1:7420/ in the background. It stops by itself after 30 minutes without use.
```

こうして起動したサーバーは、ブラウザーでカンバンを開いている間、エージェントが待っている間、スマホが接続している間は動き続けます。どれもない状態が30分続くと止まります。`AGPJ_IDLE_TIMEOUT`（60〜86,400秒）で変えられます。ポート7420が使われていれば、空いているポートを使い、そのことを表示します。`AGPJ_AUTOSTART=0`で自動の起動を止められます。このとき、サーバーが要るコマンドは、サーバーがないときと同じく`board_unavailable`で失敗します。`add`・`mv`・`ls`などのほかのコマンドは、サーバーを起動しません。

### カンバンをブラウザーで開く {#open-the-board-in-the-browser}

裏で起動したサーバーは、起動のきっかけになったプロジェクトのカンバンを、既定のブラウザーで一度だけ開きます。開く前に3秒ほど待ち、前から開いていたカンバンのタブがつながり直したとき（スリープから戻ったときなど）は何も開きません。サーバーが動いている間に質問が届いても、ブラウザーは開きません。`agpj serve`と`agpj serve -d`も開きません。

次のときは、ブラウザーを開きません。

- `AGPJ_OPEN_BROWSER=0`を設定しているとき
- SSH越しにコマンドを実行しているとき（`SSH_CONNECTION`か`SSH_TTY`が設定されている）
- Linuxで画面がないとき（`DISPLAY`も`WAYLAND_DISPLAY`も設定されていない）

開いたかどうかと、開かなかった理由は、データフォルダの`serve.log`に残ります。

自分でカンバンを開くには、`agpj open`を実行します。今のフォルダに登録されたプロジェクトか、`--project`で指定したプロジェクトのカンバンを開きます。サーバーが動いていなければ、先に裏で起動します。

```sh
agpj open
agpj --project "My project" open
agpj open --print
```

開いたときも、`http://127.0.0.1:7420/?project=<id>`のようなカンバンのアドレスを表示します。`--print`はアドレスを表示するだけです。`--json`では`{"url":"…","projectId":"…","opened":true}`を出します。ブラウザーを開かなかったときは`opened`が`false`になり、`notice`に理由が入ります。`AGPJ_AUTOSTART=0`でサーバーが動いていなければ`board_unavailable`で失敗します。開くためのプログラムを起動できなければ`browser_open_failed`で失敗し、自分で開くアドレスをメッセージに示します。

macOSでは`open`、Linuxでは`xdg-open`を使います。ほかのプログラムを使うには、`AGPJ_BROWSER_OPENER`にそのプログラムを設定します。アドレスだけを引数にして実行し、SSH越しや画面がないときも使います。

### 自分でサーバーを起動する

動かし続けたいとき、ポートを選びたいとき、起動と同時にスマホを接続したいときは、自分で起動します。

`agpj serve`は、`127.0.0.1`でAgent Projectのサーバーを起動します。`--port`でポートを選べます（既定は7420）。サーバーの起動と同時にngrokでスマホを接続するには、`--monitor ngrok`を付けます。

| オプション | 意味 |
| --- | --- |
| `--monitor ngrok` | リモートアクセスの接続を始めます。指定できるのは`ngrok`だけです。環境変数`NGROK_AUTHTOKEN`が必要で、トークンを引数で渡すことはできません。開始のたびに同じアドレスにするには、ngrokアカウントが持つドメインをダイアログに入力するか、環境変数`AGPJ_NGROK_DOMAIN`に設定するか、[`agpj setup remote`](#save-the-remote-access-settings)でコマンドラインから保存します。[同じアドレスを使い続ける](../watch/remote-access#同じアドレスを使い続ける)を参照してください。 |

スマホは、起動ごとに作る4桁の合言葉で守られます。サインイン用のオプションはありません。`--monitor`の不正な値、`NGROK_AUTHTOKEN`がないこと、未知のオプションは、終了コード2で終了します。接続を開始できなければ、`serve`は起動せず終了コード1で終了します。

接続の準備ができると、起動行にスマホ用のアドレスと合言葉が表示されます: `Serving at http://127.0.0.1:7420/; remote access at https://… with passcode 4821`。`--json`では、起動時の行にローカルの`url`と`monitor`が入ります（`mode: "ngrok"`と、`ngrok`のスマホ用`url`・`passcode`）。合言葉を出力するのはここと、カンバンの接続ダイアログだけです。トークンとポリシーは出力しません。カンバンの**設定**からも接続を設定できます。詳しくは[リモートアクセスのガイド](../watch/remote-access)を参照してください。

### バックグラウンドで実行する

`agpj serve -d`（または`--daemon`）は、サーバーをバックグラウンドで起動し、準備ができた時点で戻ります。ターミナルは空き、閉じてもサーバーは止まりません。表示されるのは通常の起動時と同じアドレス（`--monitor ngrok`ではスマホのアドレスと合言葉も）に加えて、プロセスIDとログファイルです。`--port`と`--monitor`は通常どおり使えます。`--json`では、起動時のオブジェクトに`pid`と`log`も入ります。

```sh
agpj serve -d
agpj stop
```

`agpj stop`は、実行中のサーバーを終了し、終了したところで戻ります。`serve`・`serve -d`・コマンドのどれで起動したサーバーも止められます。`stop`の後は、待つコマンド、`serve`、`serve -d`が起動するまで、Hookはサーバーを起動しません。サーバーが動いていなければ、その旨を表示して終了コード0で終わります。`stop`に`--project`は指定できません。

### スマホ用のアドレスと合言葉を渡す

ngrok接続が動いている間、`--project`を付けた`agpj show remote`は、接続の状態、公開アドレス、4桁の合言葉を表示します。接続ダイアログを開かなくても、エージェントからリンクを渡してもらえます。

```sh
agpj --project "My project" show remote
agpj --project "My project" show remote --json
```

```text
ngrok: ready
URL: https://….ngrok-free.app/?project=<id>
Passcode: 4821
Project: My project
Enabled: yes
Domain: example.ngrok-free.dev
Token: saved in the keychain
```

アドレスはそのプロジェクトを開きます。`--json`では`{"status":"ready","url":"…","entrance":"…","passcode":"…","project":{"id":"…","name":"…"},"enabled":true,"domain":"…","tokenSaved":true,"keychainAvailable":true}`を出します。`entrance`はプロジェクトを含まないアドレスで、末尾の4項目は、次に説明する保存済みの設定です。

プラグインには、このためのスキル`remote`（Claude Codeでは`/agent-project:remote`）が含まれます。エージェントに「リモートアクセスのURL」や「スマホで開きたい」と頼むと、フォルダのプロジェクトに対して`agpj --project "<名前>" show remote --json`を実行し、アドレスと合言葉を返します。サーバーやngrokは起動しません。

`--project`を付けた`show remote`は何も起動しません。サーバーが動いていないか、接続の準備ができていないときは、アドレスも合言葉も表示せず、理由を伝えて、`remote_unavailable`として終了コード1で終わります。存在しないプロジェクトは終了コード4です。

`--project`を付けない`show remote`は、保存済みのリモートアクセスの設定と接続の状態を表示し、接続が止まっていても失敗しません。

```sh
agpj show remote
agpj show remote --json
```

```text
Enabled: yes
Domain: example.ngrok-free.dev
Token: saved in the keychain
Connection: ready at https://….ngrok-free.dev
```

リモートアクセスがオンかどうか、保存してある固定ドメイン（なければ、開始のたびにngrokがアドレスを選ぶこと）、トークンがキーチェーンに保存されているか、キーチェーンが使えるかを表示します。トークン自体は表示されず、`--project`がなければ合言葉も表示されません。`--json`では、`enabled`、`domain`、`tokenSaved`、`keychainAvailable`、`status`（サーバーが動いていなければ`no_server`、動いていれば`ready`、`disabled`、`failed`などの接続の状態）、`entrance`（接続が`ready`のときはそのアドレス、それ以外は`null`）を出します。`show <id>`は、これまでどおりタスクを表示します。`show`が、タスクIDの代わりに受け付ける語は`remote`だけです。

### リモートアクセスの設定を保存する {#save-the-remote-access-settings}

`agpj setup remote`は、カンバンの接続ダイアログも保存しているリモートアクセスの設定、つまり接続をオンにするか、スマホがngrokとTailscaleのどちらで接続するか、ngrokの場合は固定ドメインとトークンを保存します。設定はデータフォルダ全体のものなのでプロジェクトは不要で、サーバーが動いていてもいなくても使えます。動いているカンバンには、再読み込みしなくても変更が反映されます。

```sh
agpj setup remote
agpj setup remote --provider tailscale --enabled true
agpj setup remote --domain example.ngrok-free.dev --enabled true
agpj setup remote --clear-domain
printf '%s' "$NGROK_AUTHTOKEN" | agpj setup remote --token-stdin
```

ターミナルでオプションなしの`agpj setup remote`を実行すると、保存済みの内容（providerと、ドメインと、トークンが保存されているかどうか。トークン自体は表示しません）を見せたうえで、スマホがngrokとTailscaleのどちらで接続するかを尋ねます。続いて、選んだ方法の設定だけを尋ねます。ngrokではドメインとトークン、Tailscaleは両方とも要らないので、これ以上は尋ねません。トークンは入力が表示されず、出力されることもありません。各質問は空のまま進めると飛ばせます。すでに保存してある値は、変えると答えたときだけ変更します。ターミナルがないときや`--json`を付けたときは何も尋ねず、終了コード`2`で終わるので、スクリプトが入力待ちで止まることはありません。

Tailscaleを保存するとき（選んだときや`--provider tailscale`のとき）は、このコンピューターのTailscaleの状態も確認し、次にすることを表示します。インストールする（Macアプリか、Homebrewのコマンドラインツール。この2つは併用できません）、サインインする、MagicDNSとHTTPS証明書を有効にする、のいずれかです。すべて整っていれば、スマホが開く`ts.net`のアドレスを表示します。この確認は案内だけで、保存を止めることはなく、コマンドは終了コード`0`で終わります。`--json`では、`status`（`ready`、`not_installed`、`not_logged_in`、`https_disabled`のいずれか）と、整っているときの`domain`を持つ`tailscale`オブジェクトが加わります。

次のオプションは、何も尋ねずに保存します。

| オプション | 意味 |
| --- | --- |
| `--provider ngrok\|tailscale` | スマホの接続方法を保存します。既定は`ngrok`です。`tailscale`ではドメインとトークンを使わず、`setup remote`もこの2つを表示しません。ほかの名前は終了コード2で終わります。[ngrokとTailscaleを選ぶ](../watch/remote-access#choose-a-way)を参照してください。 |
| `--domain <host>` | ngrokアカウントが持つ固定ドメインを保存します。ホスト名だけを指定します。小文字にそろえ、`https://`、パス、ポートを含む値は`invalid_domain`として終了コード2で終わります。 |
| `--clear-domain` | 保存してあるドメインを削除します。以後は開始のたびにngrokがアドレスを選びます。`--domain`とは同時に指定できません。 |
| `--enabled true\|false` | 接続をオンにするかどうかを保存します。 |
| `--token-stdin` | ngrokのトークンを標準入力から読み、OSのキーチェーンに保存します。カンバンの**このPCにトークンを保存する**と同じ保管場所です。トークンを引数で渡すことはできません。キーチェーンが使えないときは何も保存せず、`keychain_unavailable`として終了コード1で終わります。 |

保存しても、接続の開始、停止、置き換えはしません。すでに動いている接続はそのまま動き続け、保存した設定は次に接続を開始するときから使われます。コマンドもそのことを表示します。ブラウザーとコマンドは同じ設定ファイルを参照します。`setup remote`は`--json`を受け付け、`--project`は指定できません。`setup`のオプションとは組み合わせられません。

サーバーは合言葉をこのPCのローカルアドレスにだけ返すので、合言葉はコマンドを実行した人にだけ表示され、ngrokを通ることはありません。エージェントがチャットに貼り付けると、そのチャットを読める人は、接続が続く間カンバンを開けます。使い終わったら接続を止めてください。[リモートアクセスのガイド](../watch/remote-access)を参照してください。

バックグラウンドのサーバーは、出力をデータフォルダの`serve.log`に書きます。スマホの合言葉が入るため、このファイルは自分だけが読めます。`serve -d`のたびに作り直されます。バックグラウンドのサーバーを起動できない場合（ポートが使用中など）は、理由を表示して終了コード1で終わります。再起動後に自動で立ち上がることはありません。

動いているサーバーをコマンドが起動していた場合、`serve -d`は新しく起動せず、そのサーバーを動かし続けるようにします。自分で止まることはなくなり、つながっているものも切れません。動いているサーバーと食い違う`--port`や`--monitor`を指定しないことが条件です。前面の`agpj serve`は`already_serving`で終わり、`serve -d`を案内します。

自分で起動したサーバーがすでに動いている場合や、指定が食い違う場合、`serve -d`はそのサーバーを止めて起動し直すかどうかをターミナルで確認します。`y`と答えると置き換えます。新しいサーバーは、前のサーバーの設定ではなく、このコマンドの`--port`と`--monitor`で起動します。`--monitor ngrok`なら合言葉も新しくなります。それ以外の答えでは、動いているサーバーはそのままで、終了コード0で終わります。`--json`を付けたときや、ターミナルの外で実行したときは確認せず、終了コード1（`already_serving`）で終わります。

## setupとupdate {#setup-and-update}

`setup`は、データフォルダを作成し、Codex、Claude Code、または両方にAgent Projectプラグイン（SkillとHook）を導入します。`update`は、導入済みのプラグインをコマンドと同じバージョンにそろえます。尋ねる内容と変更するものは、[エージェントとの連携](../delegate/agents#set-up-your-agents)にあります。

```sh
agpj setup
agpj setup --agent codex --dry-run
agpj setup --agent all --yes --json
agpj update
agpj update --yes
```

| コマンド | 動作 | 終了コード |
| --- | --- | --- |
| `agpj setup` | データフォルダを作成し、設定するエージェントを尋ね、計画を表示します。`y`と答えると、プラグインを導入し、エージェントが共有フォルダを使えるようにし、CodexでAgent ProjectのHookを信頼済みにします。プロジェクトは不要です。 | `0`は成功、変更なし、取り消し。`1`はエージェントの設定に失敗した、または設定できるエージェントがなかった。`2`はコマンドラインが不正、またはターミナルがないのに必要なオプションがない。 |
| `agpj setup remote` | リモートアクセスの設定（ngrokのドメイン、オンかどうか、トークン）を保存します。プロジェクトは不要です。[リモートアクセスの設定を保存する](#save-the-remote-access-settings)を参照してください。 | `0`は成功。`1`はキーチェーンまたは設定ファイルを使えない。`2`はコマンドラインが不正、ドメインが不正、またはターミナルがないのにオプションがない。 |
| `agpj update` | プラグインが導入されたすべてのエージェントで、プラグインをこのバージョンにそろえ、新しいCodexのHookを信頼済みにします。先に計画を表示して確認します。更新後に必要なのは、自動更新ではできないとき（Codexのhookの信頼）か、プラグインを直したいときだけです。[プラグインを更新する](../delegate/agents#update-the-plugin)を参照してください。 | `0`は成功、更新なし、取り消し。`1`はエージェントの更新に失敗した。`2`はコマンドラインが不正、またはターミナルがないのに必要なオプションがない。 |

`setup`がプラグインを入れ直すのは、プラグインがない、またはファイルが`agpj`の同梱のものと違うときだけです。`update`は常に入れ直します。

どちらのコマンドも、次のオプションを受け付けます。

- `--agent codex|claude|all`で対象のエージェントを選びます。指定しないと、`setup`はターミナルで尋ね、`update`はプラグインがあるすべてのエージェントを対象にします。
- `--yes`は、CodexのHookの信頼も含めて、あらかじめ同意します。
- `--dry-run`は計画を表示するだけで、何も導入、作成、変更しません。
- `--json`は機械可読の出力を表示します。

ターミナルでは、オプションなしの`agpj setup`は、プラグインを導入したあとにリモートアクセスの設定も勧めます。設定するかどうかを尋ね（既定は「いいえ」）、設定するときは`agpj setup remote`と同じ質問をします。飛ばせば、それ以上は尋ねません。ターミナルがないとき、`--json`、`--yes`、`--dry-run`を付けたときは勧めません。

どちらもパイプを待ちません。ターミナルがないときや`--json`を付けたときは、`setup`には`--agent`が、両方に`--yes`か`--dry-run`が必要です。足りないと終了コード`2`で終わります。コマンドが`PATH`上にないエージェントは、理由を示して飛ばします。失敗の理由はエラー出力に出ます。変更する設定ファイルとそのバックアップは[データとバックアップ](../operations/data#共有フォルダをセットアップする)を参照してください。

## Skillの本文 {#skill-text}

`agpj skills`は、`agent-project`スキルの指示を出力します。`agpj skills <topic>`は、参照や雛形を1つ出力します。トピックは`project`、`language`、`files`、`requests`、`work-loop`、`questions`、`waiting`、`file-format`、`listening`、`resident`、`planning`、`fixed-epic-bodies`、`drafts`、`phone`、`task-template`、`epic-template`、`bug-epic-template`、`enhancement-epic-template`です。プラグインの`SKILL.md`が`agpj skills`を実行するので、エージェントは、インストールした`agpj`の指示をいつも受け取ります。このコマンドはMarkdownを出力し、何も書き込まず、サーバーも起動せず、常に終了コード`0`で終わります。トピックにない名前は、トピックの一覧を出力します。

## 出力と確認

JSON形式の出力が必要なときは`--json`を付けます。helpのテキスト・エラー・JSONの値は、ブラウザの言語に関わらず、常に英語です。

タスク・エピック・プロジェクトの削除と、`init`によるプロジェクトの初期化には、端末での人の確認が必要です。エージェントは、人に頼まなければなりません。

## 次に読む

- [エージェントとの連携](../delegate/agents): 各コマンドを使う場面と、作業全体の流れ。
- [トラブルシューティング](../operations/troubleshooting): エラーコードの意味。
