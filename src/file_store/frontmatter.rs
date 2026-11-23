//! TASK.md acceptance frontmatter and EPIC.md task-order metadata:
//! the body that follows it, and the composition that writes both back. TASK.md also holds
//! the `questions` list and the `plannedVersion` name.
//!
//! A file starts with a `---` line, YAML, and a closing `---` line. TASK.md reads
//! `acceptance`; EPIC.md interprets `waves`. Other keys are kept as they are. A file without
//! the opening line is a body with no conditions. Defects are reported with the
//! item they concern, and the product never rewrites a file to fix them.

mod acceptance;
mod planned;
mod questions;
mod result_plans;
mod split;
mod waves;

pub use self::acceptance::*;
pub use self::planned::*;
pub use self::questions::*;
pub use self::result_plans::*;
pub use self::split::*;
pub use self::waves::*;
