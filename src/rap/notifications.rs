use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{NotificationId, RapError, RapStore, WorkId, fetch_work};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    QuestionNeeded,
    SafetyHold,
    ResultReady,
    Unrecoverable,
}

impl NotificationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::QuestionNeeded => "question_needed",
            Self::SafetyHold => "safety_hold",
            Self::ResultReady => "result_ready",
            Self::Unrecoverable => "unrecoverable",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "question_needed" => Ok(Self::QuestionNeeded),
            "safety_hold" => Ok(Self::SafetyHold),
            "result_ready" => Ok(Self::ResultReady),
            "unrecoverable" => Ok(Self::Unrecoverable),
            _ => Err(rusqlite::Error::InvalidColumnType(
                2,
                "kind".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }

    pub fn is_safety_critical(self) -> bool {
        matches!(self, Self::SafetyHold | Self::Unrecoverable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationState {
    Pending,
    BatchedForSleep,
    Sent,
    Failed,
}

impl NotificationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::BatchedForSleep => "batched_for_sleep",
            Self::Sent => "sent",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "batched_for_sleep" => Ok(Self::BatchedForSleep),
            "sent" => Ok(Self::Sent),
            "failed" => Ok(Self::Failed),
            _ => Err(rusqlite::Error::InvalidColumnType(
                7,
                "state".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewNotification<'a> {
    pub work_id: WorkId,
    pub kind: NotificationKind,
    pub target_route: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub now: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapNotification {
    pub id: NotificationId,
    pub work_id: WorkId,
    pub kind: NotificationKind,
    pub target_route: String,
    pub title: String,
    pub body: String,
    pub safety_critical: bool,
    pub state: NotificationState,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub delivered_at: Option<i64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DispatchSummary {
    pub sent: usize,
    pub failed: usize,
}

impl RapStore {
    /// Queues a post-commit notification pointing at a specific question, result, or recovery route.
    /// Non-safety notifications created during Sleep are batched rather than sent immediately.
    pub fn enqueue_notification(
        &self,
        input: NewNotification<'_>,
    ) -> Result<RapNotification, RapError> {
        if input.target_route.trim().is_empty()
            || input.title.trim().is_empty()
            || input.body.trim().is_empty()
        {
            return Err(RapError::InvalidInput(
                "notification route, title, and body are required",
            ));
        }
        self.write(|tx| {
            fetch_work(tx, input.work_id)?;
            let sleeping: bool =
                tx.query_row("SELECT active FROM sleep_mode WHERE id = 1", [], |r| {
                    r.get(0)
                })?;
            let safety_critical = input.kind.is_safety_critical();
            let state = if sleeping && !safety_critical {
                NotificationState::BatchedForSleep
            } else {
                NotificationState::Pending
            };
            let id: i64 = tx.query_row(
                "INSERT INTO rap_notifications
                    (work_id, kind, target_route, title, body, safety_critical, state, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) RETURNING id",
                params![
                    input.work_id.0,
                    input.kind.as_str(),
                    input.target_route,
                    input.title,
                    input.body,
                    safety_critical,
                    state.as_str(),
                    input.now,
                ],
                |row| row.get(0),
            )?;
            fetch_notification(tx, NotificationId(id))
        })
    }

    /// Delivers all pending notifications. Delivery failures are recorded on the notification
    /// ledger without altering board/work state.
    pub fn dispatch_pending_notifications(
        &self,
        now: i64,
        sender: &dyn Fn(&RapNotification) -> Result<(), String>,
    ) -> Result<DispatchSummary, RapError> {
        let pending = self.read(|connection| {
            let mut stmt = connection.prepare(
                "SELECT id, work_id, kind, target_route, title, body, safety_critical,
                        state, attempts, last_error, created_at, delivered_at
                 FROM rap_notifications WHERE state = 'pending' ORDER BY id",
            )?;
            let rows = stmt.query_map([], read_notification)?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
        })?;

        let mut summary = DispatchSummary::default();
        for notification in pending {
            match sender(&notification) {
                Ok(()) => {
                    self.write(|tx| {
                        tx.execute(
                            "UPDATE rap_notifications
                             SET state = 'sent', attempts = attempts + 1,
                                 last_error = NULL, delivered_at = ?1
                             WHERE id = ?2",
                            params![now, notification.id.0],
                        )?;
                        Ok(())
                    })?;
                    summary.sent += 1;
                }
                Err(error) => {
                    self.write(|tx| {
                        tx.execute(
                            "UPDATE rap_notifications
                             SET state = 'failed', attempts = attempts + 1, last_error = ?1
                             WHERE id = ?2",
                            params![error, notification.id.0],
                        )?;
                        Ok(())
                    })?;
                    summary.failed += 1;
                }
            }
        }
        Ok(summary)
    }

    /// Releases notifications batched during Sleep so they can be summarized or sent on wake.
    pub fn flush_sleep_notifications(&self) -> Result<Vec<RapNotification>, RapError> {
        self.write(|tx| {
            let batched = {
                let mut stmt = tx.prepare(
                    "SELECT id, work_id, kind, target_route, title, body, safety_critical,
                            state, attempts, last_error, created_at, delivered_at
                     FROM rap_notifications WHERE state = 'batched_for_sleep' ORDER BY id",
                )?;
                let rows = stmt.query_map([], read_notification)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            tx.execute(
                "UPDATE rap_notifications SET state = 'pending' WHERE state = 'batched_for_sleep'",
                [],
            )?;
            Ok(batched)
        })
    }

    pub fn notification(&self, id: NotificationId) -> Result<RapNotification, RapError> {
        self.read(|connection| fetch_notification(connection, id))
    }
}

fn fetch_notification(
    connection: &Connection,
    id: NotificationId,
) -> Result<RapNotification, RapError> {
    connection
        .query_row(
            "SELECT id, work_id, kind, target_route, title, body, safety_critical,
                    state, attempts, last_error, created_at, delivered_at
             FROM rap_notifications WHERE id = ?1",
            [id.0],
            read_notification,
        )
        .optional()?
        .ok_or(RapError::NotFound("notification"))
}

fn read_notification(row: &Row<'_>) -> rusqlite::Result<RapNotification> {
    let raw_kind: String = row.get(2)?;
    let raw_state: String = row.get(7)?;
    Ok(RapNotification {
        id: NotificationId(row.get(0)?),
        work_id: WorkId(row.get(1)?),
        kind: NotificationKind::parse(&raw_kind)?,
        target_route: row.get(3)?,
        title: row.get(4)?,
        body: row.get(5)?,
        safety_critical: row.get(6)?,
        state: NotificationState::parse(&raw_state)?,
        attempts: row.get(8)?,
        last_error: row.get(9)?,
        created_at: row.get(10)?,
        delivered_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{BoardLane, NewWork, WorkKind};

    #[test]
    fn batches_non_safety_during_sleep_sends_safety_and_preserves_board_on_failure() {
        let root = tempfile::tempdir().unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let work = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: root.path(),
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Notify test",
                rank: 1,
            })
            .unwrap()
            .id;

        store.enter_sleep(100).unwrap();

        // Routine result notification is batched during Sleep.
        let routine = store
            .enqueue_notification(NewNotification {
                work_id: work,
                kind: NotificationKind::ResultReady,
                target_route: &format!("/rap/work/{}/results/1", work.0),
                title: "Result ready",
                body: "Draft ready",
                now: 101,
            })
            .unwrap();
        assert_eq!(routine.state, NotificationState::BatchedForSleep);

        // Safety-critical notification stays pending immediately even during Sleep.
        let safety = store
            .enqueue_notification(NewNotification {
                work_id: work,
                kind: NotificationKind::SafetyHold,
                target_route: &format!("/rap/work/{}/questions/1", work.0),
                title: "Approval required",
                body: "Irreversible action held",
                now: 102,
            })
            .unwrap();
        assert_eq!(safety.state, NotificationState::Pending);

        // Transport failure marks notification failed without altering board state.
        let failed = store
            .dispatch_pending_notifications(103, &|_| Err("push endpoint 503".into()))
            .unwrap();
        assert_eq!(failed.failed, 1);
        assert_eq!(failed.sent, 0);
        assert_eq!(
            store.notification(safety.id).unwrap().state,
            NotificationState::Failed
        );
        assert_eq!(store.work(work).unwrap().lane, BoardLane::Backlog);

        // Flushing Sleep releases batched notifications for wakeup delivery.
        let flushed = store.flush_sleep_notifications().unwrap();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0].id, routine.id);
        let sent = store
            .dispatch_pending_notifications(200, &|_| Ok(()))
            .unwrap();
        assert_eq!(sent.sent, 1);
        assert_eq!(
            store.notification(routine.id).unwrap().state,
            NotificationState::Sent
        );
    }
}
