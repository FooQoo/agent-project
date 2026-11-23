use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Column {
    /// A plan that is saved but not yet handed over to be worked on.
    Backlog,
    Todo,
    Doing,
    Done,
}

impl Column {
    /// Lowercase machine form (`todo`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Column::Backlog => "backlog",
            Column::Todo => "todo",
            Column::Doing => "doing",
            Column::Done => "done",
        }
    }

    /// Capitalized display form (`Todo`).
    pub fn label(&self) -> &'static str {
        match self {
            Column::Backlog => "Backlog",
            Column::Todo => "Todo",
            Column::Doing => "Doing",
            Column::Done => "Done",
        }
    }
}

impl FromStr for Column {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "backlog" => Ok(Column::Backlog),
            "todo" => Ok(Column::Todo),
            "doing" => Ok(Column::Doing),
            "done" => Ok(Column::Done),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Not running: every task outside Doing, and an AI task in Doing that is not being worked.
    Idle,
    /// An AI task in Doing that an agent is working on.
    Running,
    /// An AI task in Doing whose work is held, for example while it waits for an answer.
    Paused,
}

impl RunState {
    /// Lowercase machine form (`running`).
    pub fn as_str(&self) -> &'static str {
        match self {
            RunState::Idle => "idle",
            RunState::Running => "running",
            RunState::Paused => "paused",
        }
    }

    /// Whether an agent may take the task up: it is idle or paused.
    pub fn can_start(&self) -> bool {
        matches!(self, RunState::Idle | RunState::Paused)
    }

    /// Capitalized display form (`Running`).
    pub fn label(&self) -> &'static str {
        match self {
            RunState::Idle => "Idle",
            RunState::Running => "Running",
            RunState::Paused => "Paused",
        }
    }
}

impl FromStr for RunState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(RunState::Idle),
            "running" => Ok(RunState::Running),
            "paused" => Ok(RunState::Paused),
            _ => Err(()),
        }
    }
}
