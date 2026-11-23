---
title: バージョン別レポートとテーマ別の振り返り
---

# バージョン別レポートとテーマ別の振り返り

あるバージョンで何を届けたか、何が残っているか、複数のバージョンにまたがる目標がどこまで進んだかをエージェントに尋ねるときの手順です。エージェントはどちらのレポートもプロジェクトの記録から組み立て、各記述を元のカード、成果、履歴までたどれるようにします。エージェントが従うスキルは`agpj skills release-report`で表示されます。

## 一つのバージョンのレポート

バージョンのレポートを頼むと、エージェントはそのバージョンに[`release doc`](../reference/cli#generate-a-release-document)を実行して`docs/release/VERSION.md`を書き、生成ブロックの周りに要約を書きます。

生成ブロックには、そのバージョンに予定したカード、検証済み、受入条件なしで完了したカード、残作業、別のバージョンへ移したカードを、カード番号と`agpj show <id>`への案内付きで載せます。次の3点で記述の根拠を保ちます。

- バージョンから外れたカードは履歴から拾い、移動先とともに一度だけ載せます。アーカイブ済みのカードも数えます。
- 検証済みは、受入条件があり、そのすべてを満たしたカードです。受入条件がないDoneのカードは検証済みではなく、残作業にも残しません。目的を満たしたかを示すものがないため、「受入条件なしで完了」として別に載せます。
- 「別のバージョンへ移した」には、後のバージョン、前のバージョン、バージョンなしのいずれに移した場合も、バージョンから外れたすべてのカードを載せ、各行に移動先を書きます。
- 公開済みは別の事実として扱います。次の節で説明します。

### 公開済みと数える条件

カードがDoneでも、公開済みとは限りません。Agent Projectは、成果の本文に`Published: バージョン名`の行があり、公開物（リリースページやパッケージなど）へのリンクが付いている場合だけ、そのバージョンで公開済みと数えます。

```sh
agpj --project "My project" result add 12 --actor ai --final --title "Released in v1.0.0" \
  --summary "Released.
Published: v1.0.0" --link https://example.com/releases/v1.0.0
```

行はあってもリンクがない成果は主張にすぎないので、レポートは「確認できない」に載せます。そうした成果がないカードも、完了していても確認できません。

明示的な形式なら、バージョン名を検証し、リンクを必須にし、`Published:`の行も自動で付けます。

```sh
agpj --project "My project" result add 12 --actor ai --published v1.0.0 \
  --link https://example.com/releases/v1.0.0 --summary "Released."
```

公開の記録は、Doneのカードにも、最終成果がすでにあるカードにも、いつでも追加できます。Doneのカードが受け付けるのはこの記録だけで、ほかの成果・本文・受け入れ条件・列は変わりません。Web APIでは`POST /api/projects/{projectId}/tasks/{id}/results`で同じ記録を追加できます。詳しくは[Web API](../../dev/reference/web-api)を参照してください。

### 例：バージョンの一部だけ公開を確認できた場合

次のブロックは、検証済みの2枚のうち1枚にはリンク付きの記録があり、もう1枚は公開と書いているだけのデモプロジェクトから生成したものです。

```markdown
## Release status of v1.0.0

- Planned: 2
- Verified: 2
- Done without acceptance conditions: 0
- Remaining: 0
- Moved to another version: 1
- Published (with a publication record): 1 / 2

### Planned

- #3 Queue edits while offline (done; acceptance 1/1)
- #5 Open the last board without network (done, archived; acceptance 1/1)

### Verified

- #3 Queue edits while offline (done; acceptance 1/1)
  - Source: `agpj show 3`, results 1, history entries 6
  - Result: Offline queue released [final] (https://example.com/releases/v1.0.0)
- #5 Open the last board without network (done, archived; acceptance 1/1)
  - Source: `agpj show 5`, results 1, history entries 6
  - Result: Offline board cache done [final]

### Done without acceptance conditions

No done task lacks acceptance conditions.

### Remaining

Nothing remains.

### Moved to another version

- #4 Merge conflicting edits on reconnect (todo; acceptance 1/2)
  - Source: `agpj show 4`, results 0, history entries 4
  - Moved to: v1.1.0

### Publication

A task counts as published only when one of its results states `Published: VERSION` on its own line and links to what was published. Done alone is not publication. Generating this document publishes nothing.

### Published

- #3 Queue edits while offline (done; acceptance 1/1)
  - Record: `agpj show 3`, result 1 (https://example.com/releases/v1.0.0)

### Not confirmed as published

- #5 Open the last board without network (done, archived; acceptance 1/1)
  - Unconfirmed: result 1 states the publication but has no link
```

レポートが「1 / 2」とするのは、公開記録の裏付けがあるのが#3だけだからです。#5は完了して検証済みですが、公開へのリンクがないため、公開済みとは書きません。

## テーマの振り返り

テーマは、専用の項目を設けずに進めている目標（例：「オフライン対応」）です。エージェントにテーマ名を伝えると、エージェントはそれに属するエピックとカードを探し、取り込んだものを示したうえで、次を照合します。

1. 各エピックの達成条件（`agpj epic show ID`）。
2. 各条件に関わるカードの受入条件、検証記録、成果、履歴。バージョンをまたぎ、アーカイブ済みのカードやバージョンを移ったカードも含めます。

各条件は、検証済み、進んだが未達、未着手、未確認のいずれかとし、判定ごとにカードと成果を示します。振り返りには、バージョンごとに進んだ点と、未確認事項とそれを確かめる方法を載せます。カードや条件の件数から達成率や進捗の割合を推測せず、`EPIC.md`の条件に自分でチェックも付けません。付けるのは条件を持つ人です。

### 例：2つのバージョンと2つのエピックにまたがるテーマ

デモプロジェクトからエージェントが書いた振り返りです。テーマは「オフライン対応」で、エピック`SYNC`と`CACHE`、バージョンv1.0.0とv1.1.0にまたがります。

```markdown
# 振り返り：オフライン対応（v1.0.0からv1.1.0）

対象：テーマ「オフライン対応」。プロジェクトでは2つのエピック、`SYNC`（オフライン同期）と`CACHE`（ローカルキャッシュ）に分かれ、バージョンv1.0.0とv1.1.0にまたがります。根拠：`agpj epic show SYNC`、`agpj epic show CACHE`、#3から#7の`agpj show <id>`、生成したレポート`docs/release/v1.0.0.md`と`docs/release/v1.1.0.md`。以下の各行は、元のカード、成果、履歴を示します。達成率は出しません。テーマの達成条件は4つあり、それぞれを自分の根拠だけで判定します。

## エピックSYNC：オフライン同期

| 達成条件（`epic show SYNC`） | 状態 | 根拠 |
| --- | --- | --- |
| オフラインで行った編集が、再接続後に失われずサーバーへ届く。 | 進み、検証済み | #3「オフライン中の編集を溜める」：AC-1完了。検証記録に、5分の断線後に溜めた200件を再生して欠落がなかったとある。最終成果1はv1.0.0のリリースページにリンクし`Published: v1.0.0`と書く。 |
| 競合した編集が、どちらも失われずに解決される。 | 進んだが未達 | #4「再接続時に競合する編集を統合する」：AC-1（双方の編集を統合）は完了、AC-2（編集が黙って消えない）は未完了。検証記録に、重なる編集はまだ片方を採るとある。成果なし。履歴：v1.0.0の予定から、v1.1.0へ移した（`Changed planned version from v1.0.0 to v1.1.0.`）。 |

## エピックCACHE：ローカルキャッシュ

| 達成条件（`epic show CACHE`） | 状態 | 根拠 |
| --- | --- | --- |
| 最後に開いたカンバンが、ネットワークなしで開く。 | 検証済み。公開は未確認 | #5「最後のカンバンをネットワークなしで開く」（アーカイブ済み）：AC-1完了。検証記録に、ネットワークを切ったブラウザテストでキャッシュしたカンバンが開いたとある。最終成果1は`Published: v1.0.0`と書くがリンクがないため、v1.0.0のレポートは公開済みと数えない。 |
| キャッシュが設定した容量上限を超えない。 | 未着手 | #6「キャッシュ容量に上限を設ける」（v1.1.0）：AC-1未完了。検証記録は「未着手」。成果なし。 |

4つの条件に含まれませんが、#7「オフラインバナーを出す」（CACHE、v1.1.0）は検証済み（AC-1完了、PC幅とスマホ幅で確認）で、v1.1.0向けのリンク付き公開記録（成果1）があります。どちらのエピックの条件にも名前が出ないため、どの条件の進捗にも数えません。

## バージョンごとに進んだ点

- **v1.0.0**（レポート`docs/release/v1.0.0.md`）：予定2枚がどちらも検証済み（#3、#5）、1枚が外へ移った（#4）。検証済み2枚のうち、リンク付きの公開記録があるのは1枚（#3）。
- **v1.1.0**（レポート`docs/release/v1.1.0.md`）：予定3枚。#7は検証済みでリンク付きの公開記録がある。#4と#6が残る。#4はv1.0.0から移ってきた。

## 未確認事項

- #5がv1.0.0で公開されたか。#5の成果1は公開と書くがリンクがない。リリースページやパッケージのリンクを求めるか、リンク付きの成果を記録する。それまでは未確認。
- 重なる編集でデータが失われないか。#4のAC-2は未完了で、検証記録にも片方が勝つとある。「どちらも失われずに」という条件は未達。
- 容量上限。#6には検証記録がまったくない。
- #7がテーマの条件に効くか。エピックに記述がない。条件が足りないのかどうかは人が判断する。

## 主張しないこと

- 割合は出しません。検証済みの条件が1つ、一部進んだものが1つ、未着手が1つ、検証済みだが公開未確認が1つあります。カードの大きさは揃っておらず、条件はカードの数でもないので、これらの件数ではテーマの完成度は測れません。
- どのエピックも閉じません。条件を持つ人が確認するまで、両方の`EPIC.md`の`- [ ]`はそのままです。

## 次に

- どちらもv1.1.0に予定した#4と#6が、テーマの条件を満たせるかを左右します。
```
