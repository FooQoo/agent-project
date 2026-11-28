# RAPの導入・データ移行・Tailscale切り替え

Remote Agent Project（RAP）で Codex・Claude Code を使うための準備、旧 Agent Project のファイルストアからのデータ移行、およびリモート接続を ngrok から Tailscale へ切り替える手順を説明します。

## 前提条件と名称確認

- `codex`（0.172.0 以降）および `claude`（Claude Code 2.1.x 以降）をインストールし、ローカルでサインインしておきます。
- ローカル環境で `rap` のコマンド名衝突がないこと（`which rap` 未登録）を確認済みです。正式な配布名称の最終決定までは既存の `agent-project` 配布経路を維持します。

## Tailscale によるリモート接続（ngrok からの移行）

RAP ではスマホからのリモート接続を Tailscale に統一します。

1. PC とスマホを同じ tailnet に参加させます。
2. `agpj setup remote --provider tailscale` を実行し、保存済み設定を Tailscale へ切り替えます。
3. スマホから接続し、依頼・承認・質問回答・成果確認ができること、および失効・期限切れセッションが拒否されることを確認します。
4. 旧環境の `.internal/monitor-connection.json` に ngrok のドメインやキーチェーンのトークンが残っている場合でも、`provider` を `tailscale` に切り替える（または無効化する）ことで予期せぬ ngrok トンネルが起動しない状態を保てます。Tailscale 接続の確認後に不要な ngrok トークンを OS キーチェーンから削除してください。

## 旧 Agent Project データの移行

従来のファイルストア（`shared/`）から移行する手順は次のとおりです。

1. 旧常駐セッションと `agpj serve` をすべて停止します。旧セッションが稼働中の場合、RAP は移行を拒否します。
2. ロールバックに備え、`shared/` のバックアップを SHA-256 ダイジェスト付きで取得します（`copy_and_digest_tree`）。
3. `rap.db` へ移行を実行します。移行では次を保持・照合します。
   - Epic・Task と親 Epic への所属関係
   - タスク履歴（`legacy_history`）
   - 質問と回答
   - 公開済み成果・添付ファイル（`rap.db.objects` へハッシュ保存）および ID・SHA-256 対応表（`legacy_id_map`）
   - 既存の Markdown（`EPIC.md`, `TASK.md`, `docs/release/*.md`, プロジェクト `docs/`）は削除せず保全
4. 移行直後の RAP は必ず `auto_run = false`、`max_sessions = 1` で開始します。内容を確認してから自動実行を有効にしてください。

## ロールバック手順

旧環境へ戻す場合は次の手順を実行します。

1. RAP を停止し、実行中の Attempt がないことを確認します。
2. バックアップから `shared/` を復元し、ダイジェスト照合（`digest_tree`）で完全一致を確認します。
3. 復元した `shared/` に対して旧 `agpj serve` を起動します。
