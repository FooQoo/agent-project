//! The `resultPlans` list of TASK.md: what a task will deliver as results, and the condition
//! under which each should be delivered. Plans are kept apart from the acceptance conditions:
//! a plan says what and when, a condition says what makes the task complete.

use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};

use super::split::{Split, assemble, defect, read_yaml, split};
use crate::file_store::validate::Defect;

const KEY: &str = "resultPlans";

/// The most characters the content or the condition of a plan may have (after trimming).
pub const MAX_RESULT_PLAN_TEXT_CHARS: usize = 2_000;

/// The most plans a task may hold.
pub const MAX_RESULT_PLANS: usize = 50;

/// One planned result. `id` is `RP-<n>` and never changes once written. `content` says what
/// will be delivered, `condition` says in words when it should be delivered, and `is_final`
/// marks the final result (at most one per task; the other plans are intermediate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResultPlan {
    /// Empty in input that leaves the ID to the store; the store fills it before writing.
    #[serde(default)]
    pub id: String,
    pub content: String,
    pub condition: String,
    #[serde(default, rename = "final")]
    pub is_final: bool,
}

fn field(index: usize, name: &str) -> String {
    format!("{KEY}[{index}].{name}")
}

/// The plans of a text and the defects of the list. Omitted, `null` and an empty list mean none.
/// With `require_id`, a plan without an ID is a defect (a stored file); without it, the ID may
/// be missing and is left empty for `fill_ids`. A frontmatter that cannot be read is not
/// reported here: the acceptance check reports it.
pub fn result_plans_of(text: &str, require_id: bool) -> (Vec<ResultPlan>, Vec<Defect>) {
    let text = text.replace("\r\n", "\n");
    let Split::Found { yaml, .. } = split(&text) else {
        return (Vec::new(), Vec::new());
    };
    let Ok(Value::Mapping(map)) = read_yaml(yaml) else {
        return (Vec::new(), Vec::new());
    };
    let list = match map.get(KEY) {
        None | Some(Value::Null) => return (Vec::new(), Vec::new()),
        Some(Value::Sequence(list)) => list,
        Some(_) => return (Vec::new(), vec![defect(KEY, "resultPlans must be a list.")]),
    };
    let mut plans = Vec::new();
    let mut defects = Vec::new();
    for (index, element) in list.iter().enumerate() {
        match read_plan(index, element, require_id) {
            Ok(plan) => plans.push(plan),
            Err(mut found) => defects.append(&mut found),
        }
    }
    if defects.is_empty() {
        defects.extend(check_plans(&plans, require_id));
    }
    if !defects.is_empty() {
        plans.clear();
    }
    (plans, defects)
}

fn read_plan(index: usize, element: &Value, require_id: bool) -> Result<ResultPlan, Vec<Defect>> {
    let Value::Mapping(map) = element else {
        return Err(vec![defect(
            &format!("{KEY}[{index}]"),
            "A plan must be a mapping with id, content, condition, and final.",
        )]);
    };
    let mut defects = Vec::new();
    for key in map.keys() {
        if !matches!(key.as_str(), Some("id" | "content" | "condition" | "final")) {
            let name = key.as_str().map_or_else(|| "?".to_string(), str::to_string);
            defects.push(defect(
                &field(index, &name),
                "This key is not known for a plan.",
            ));
        }
    }
    let id = match map.get("id") {
        None | Some(Value::Null) if !require_id => Some(String::new()),
        other => text_field(other, "id", &field(index, "id"), &mut defects),
    };
    let content = text_field(
        map.get("content"),
        "content",
        &field(index, "content"),
        &mut defects,
    );
    let condition = text_field(
        map.get("condition"),
        "condition",
        &field(index, "condition"),
        &mut defects,
    );
    let is_final = match map.get("final") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(_) => {
            defects.push(defect(
                &field(index, "final"),
                "final must be true or false.",
            ));
            false
        }
    };
    match (id, content, condition) {
        (Some(id), Some(content), Some(condition)) if defects.is_empty() => Ok(ResultPlan {
            id,
            content,
            condition,
            is_final,
        }),
        _ => Err(defects),
    }
}

fn text_field(
    value: Option<&Value>,
    key: &str,
    field: &str,
    defects: &mut Vec<Defect>,
) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(_) => {
            defects.push(defect(field, &format!("{key} must be text.")));
            None
        }
        None => {
            defects.push(defect(field, &format!("{key} is missing.")));
            None
        }
    }
}

fn is_plan_id(text: &str) -> bool {
    let Some(digits) = text.strip_prefix("RP-") else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && !digits.starts_with('0')
}

/// The checks shared by reading and writing: the ID's form and uniqueness, the length of each
/// text, the number of plans, and at most one final plan. An empty ID passes unless
/// `require_id`.
pub fn check_plans(plans: &[ResultPlan], require_id: bool) -> Vec<Defect> {
    let mut defects = Vec::new();
    if plans.len() > MAX_RESULT_PLANS {
        defects.push(defect(KEY, "A task may have at most 50 result plans."));
    }
    let mut seen: Vec<&str> = Vec::new();
    for (index, plan) in plans.iter().enumerate() {
        if plan.id.is_empty() {
            if require_id {
                defects.push(defect(&field(index, "id"), "id is missing."));
            }
        } else if !is_plan_id(&plan.id) {
            defects.push(defect(
                &field(index, "id"),
                "The ID must be RP- followed by a positive number without a leading zero.",
            ));
        } else if seen.contains(&plan.id.as_str()) {
            defects.push(defect(
                &field(index, "id"),
                "The ID is used by another plan.",
            ));
        } else {
            seen.push(&plan.id);
        }
        for (name, text) in [("content", &plan.content), ("condition", &plan.condition)] {
            let length = text.trim().chars().count();
            if length == 0 || length > MAX_RESULT_PLAN_TEXT_CHARS {
                defects.push(defect(
                    &field(index, name),
                    "The text must be 1 to 2,000 characters after trimming.",
                ));
            }
        }
    }
    if plans.iter().filter(|plan| plan.is_final).count() > 1 {
        defects.push(defect(KEY, "Only one plan may be the final result."));
    }
    defects
}

/// Give each plan without an ID the next free `RP-<n>`, above every number in the list.
pub fn fill_ids(plans: &mut [ResultPlan]) {
    let mut next = plans
        .iter()
        .filter_map(|plan| plan.id.strip_prefix("RP-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    for plan in plans.iter_mut().filter(|plan| plan.id.is_empty()) {
        next += 1;
        plan.id = format!("RP-{next}");
    }
}

fn plan_value(plan: &ResultPlan) -> Value {
    let mut map = Mapping::new();
    map.insert(Value::String("id".into()), Value::String(plan.id.clone()));
    map.insert(
        Value::String("content".into()),
        Value::String(plan.content.clone()),
    );
    map.insert(
        Value::String("condition".into()),
        Value::String(plan.condition.clone()),
    );
    map.insert(Value::String("final".into()), Value::Bool(plan.is_final));
    Value::Mapping(map)
}

/// Write a text's `resultPlans`. Plans without an ID get one first. The other keys and the body
/// stay, and a new key goes in front of `history`; an empty list removes the key. The plans are
/// checked before anything is written, and a frontmatter that cannot be read is an error.
pub fn compose_result_plans(original: &str, plans: &[ResultPlan]) -> Result<String, Defect> {
    let mut plans = plans.to_vec();
    fill_ids(&mut plans);
    if let Some(first) = check_plans(&plans, true).into_iter().next() {
        return Err(first);
    }
    let original = original.replace("\r\n", "\n");
    let (map, body) = match split(&original) {
        Split::None => (Mapping::new(), original.clone()),
        Split::Unclosed => {
            return Err(defect(
                "frontmatter",
                "The frontmatter has no closing --- line.",
            ));
        }
        Split::Found { yaml, body } => match read_yaml(yaml)? {
            Value::Mapping(map) => (map, body.to_string()),
            _ => unreachable!("read_yaml returns a mapping"),
        },
    };
    let list = Value::Sequence(plans.iter().map(plan_value).collect());
    let mut out = Mapping::new();
    let mut placed = false;
    for (key, item) in map {
        if key.as_str() == Some(KEY) {
            continue;
        }
        if !placed && !plans.is_empty() && key.as_str() == Some("history") {
            out.insert(Value::String(KEY.into()), list.clone());
            placed = true;
        }
        out.insert(key, item);
    }
    if !placed && !plans.is_empty() {
        out.insert(Value::String(KEY.into()), list);
    }
    assemble(out, &body)
}

/// Check the plans a text carries and give those without an ID one, as a new or edited text
/// needs before it is saved. A text without plans comes back unchanged.
pub fn normalize_result_plans(text: &str) -> Result<String, Defect> {
    let (plans, defects) = result_plans_of(text, false);
    if let Some(first) = defects.into_iter().next() {
        return Err(first);
    }
    if plans.is_empty() {
        return Ok(text.to_string());
    }
    compose_result_plans(text, &plans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(content: &str, condition: &str, is_final: bool) -> ResultPlan {
        ResultPlan {
            id: String::new(),
            content: content.into(),
            condition: condition.into(),
            is_final,
        }
    }

    #[test]
    fn assigns_stable_ids_and_round_trips() {
        let text = "---\nacceptance: []\nhistory: []\n---\n\nBody\n";
        let plans = [
            plan("Mock", "When the layout is fixed", false),
            plan("App", "When done", true),
        ];
        let written = compose_result_plans(text, &plans).unwrap();
        assert!(written.find("resultPlans").unwrap() < written.find("history").unwrap());
        let (read, defects) = result_plans_of(&written, true);
        assert!(defects.is_empty());
        assert_eq!(read[0].id, "RP-1");
        assert_eq!(read[1].id, "RP-2");
        assert!(read[1].is_final && !read[0].is_final);
        // Rewriting keeps the IDs and adds the next free one to a new plan.
        let mut more = read.clone();
        more.remove(0);
        more.push(plan("Docs", "After review", false));
        let again = compose_result_plans(&written, &more).unwrap();
        let ids: Vec<_> = result_plans_of(&again, true)
            .0
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, ["RP-2", "RP-3"]);
        assert!(again.ends_with("Body\n"));
    }

    #[test]
    fn rejects_empty_text_and_a_second_final() {
        for bad in [
            "resultPlans:\n  - {content: '', condition: c}",
            "resultPlans:\n  - {content: a, condition: '  '}",
            "resultPlans:\n  - {content: a}",
            "resultPlans:\n  - {content: a, condition: b, final: yes please}",
            "resultPlans:\n  - {content: a, condition: b, final: true}\n  - {content: c, condition: d, final: true}",
            "resultPlans:\n  - {id: RP-1, content: a, condition: b}\n  - {id: RP-1, content: c, condition: d}",
            "resultPlans:\n  - {id: AC-1, content: a, condition: b}",
            "resultPlans: nope",
        ] {
            let text = format!("---\n{bad}\n---\n");
            assert!(!result_plans_of(&text, false).1.is_empty(), "{bad}");
            assert!(normalize_result_plans(&text).is_err(), "{bad}");
        }
        assert!(
            !result_plans_of(
                "---\nresultPlans:\n  - {content: a, condition: b}\n---\n",
                true
            )
            .1
            .is_empty()
        );
    }

    #[test]
    fn no_plans_is_valid_and_untouched() {
        for text in [
            "plain",
            "---\nacceptance: []\n---\nBody\n",
            "---\nresultPlans: []\n---\n",
        ] {
            assert!(result_plans_of(text, true).1.is_empty());
            assert_eq!(normalize_result_plans(text).unwrap(), text);
        }
        let final_only = [plan("Report", "When all work is done", true)];
        let text = compose_result_plans("---\nacceptance: []\n---\n", &final_only).unwrap();
        assert_eq!(result_plans_of(&text, true).0.len(), 1);
    }
}
