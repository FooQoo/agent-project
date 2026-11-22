use super::*;

fn many(f: &Fixture, count: u32) {
    // Two per day over several day files, so the stock spans files.
    for n in 0..count {
        let when = at(2 + n / 2, 10 + (n % 2) * 6, 0);
        assert!(
            f.store
                .append_summary(&f.project, &summary(when, &["x"]))
                .unwrap()
        );
    }
}

fn line_count(f: &Fixture) -> usize {
    std::fs::read_dir(f.dir())
        .unwrap()
        .map(|e| {
            std::fs::read_to_string(e.unwrap().path())
                .unwrap()
                .lines()
                .count()
        })
        .sum()
}

#[test]
fn taking_stock_dismisses_the_oldest_beyond_ten_and_removes_no_line() {
    let f = fixture();
    many(&f, 14);
    assert_eq!(f.store.take_stock_of_summaries(&f.project).unwrap(), 4);
    assert_eq!(line_count(&f), 14);
    let all = f.store.summaries(&f.project).unwrap();
    assert_eq!(all.len(), 14);
    // Newest first: the first ten are live, the last four (the oldest) are dismissed.
    assert!(all[..10].iter().all(|s| !s.dismissed));
    assert!(all[10..].iter().all(|s| s.dismissed));
    // A second pass has nothing to do.
    assert_eq!(f.store.take_stock_of_summaries(&f.project).unwrap(), 0);
    // Already dismissed summaries do not count toward the ten.
    f.store.dismiss_summary(&f.project, &all[0].id).unwrap();
    assert_eq!(f.store.take_stock_of_summaries(&f.project).unwrap(), 0);
    assert_eq!(f.store.listed_summaries(&f.project).unwrap().len(), 9);
}

#[test]
fn taking_stock_keeps_unreadable_lines_and_listing_is_capped_at_ten() {
    let f = fixture();
    many(&f, 12);
    let path = f.dir().join("2026-11-02.ldjson");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("not json\n{text}")).unwrap();
    // Before stock is taken, the list is already capped and has no dismissed summary.
    let listed = f.store.listed_summaries(&f.project).unwrap();
    assert_eq!(listed.len(), 10);
    assert!(listed.windows(2).all(|w| w[0].to > w[1].to));
    f.store.take_stock_of_summaries(&f.project).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains("not json"));
    assert_eq!(f.store.listed_summaries(&f.project).unwrap().len(), 10);
}
