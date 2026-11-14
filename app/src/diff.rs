//! A file's edits as the lines a virtual list draws.

use logbook_core::dto::FileEditDto;

/// One flattened line of a file's edit cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffItem {
    /// The card's top: "Edit 2 of 3" and its time.
    Head { label: String, ts: Option<String> },
    /// The ⋯ between hunks.
    Gap,
    Line { no: u32, op: char, text: String },
    Note(String),
    /// The card's bottom edge.
    End,
}

/// Flattens a file's edits into the lines a virtual list draws, and whether there were none.
pub fn flatten_edits(edits: &[FileEditDto]) -> (Vec<DiffItem>, bool) {
    let mut out = Vec::new();
    for (i, e) in edits.iter().enumerate() {
        let label = if e.created {
            "Created".to_string()
        } else if edits.len() > 1 {
            format!("Edit {} of {}", i + 1, edits.len())
        } else {
            "Edit".to_string()
        };
        out.push(DiffItem::Head { label, ts: e.timestamp.clone() });
        for (hi, h) in e.hunks.iter().enumerate() {
            if hi > 0 {
                out.push(DiffItem::Gap);
            }
            let (mut old_no, mut new_no) = (h.old_start, h.new_start);
            for line in &h.lines {
                let op = match line.chars().next() {
                    Some('+') => '+',
                    Some('-') => '-',
                    _ => ' ',
                };
                // Removed lines are numbered in the old file, the rest in the new.
                let no = match op {
                    '-' => {
                        let n = old_no;
                        old_no += 1;
                        n
                    }
                    _ => {
                        let n = new_no;
                        new_no += 1;
                        if op == ' ' {
                            old_no += 1;
                        }
                        n
                    }
                };
                out.push(DiffItem::Line { no, op, text: line.chars().skip(1).collect() });
            }
        }
        if e.hunks.is_empty() {
            out.push(DiffItem::Note("An empty file.".into()));
        }
        if e.truncated {
            out.push(DiffItem::Note("Long edit: the rest isn’t shown.".into()));
        }
        out.push(DiffItem::End);
    }
    (out, edits.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use logbook_core::models::Hunk;

    fn edit(created: bool, hunks: Vec<Hunk>, truncated: bool) -> FileEditDto {
        FileEditDto { timestamp: Some("2026-11-03T10:00:00Z".into()), created, hunks, truncated }
    }

    #[test]
    fn lines_are_numbered_in_the_file_they_belong_to() {
        let h = Hunk { old_start: 7, new_start: 7, lines: vec![" keep".into(), "-gone".into(), "+new".into(), "+more".into(), " tail".into()] };
        let (items, empty) = flatten_edits(&[edit(false, vec![h], false)]);
        assert!(!empty);
        let lines: Vec<_> = items.iter().filter_map(|i| match i { DiffItem::Line { no, op, text } => Some((*no, *op, text.as_str())), _ => None }).collect();
        // The removed line is line 8 of the old file; the added ones are lines 8 and 9 of the new.
        assert_eq!(lines, [(7, ' ', "keep"), (8, '-', "gone"), (8, '+', "new"), (9, '+', "more"), (10, ' ', "tail")]);
        assert_eq!(items.first(), Some(&DiffItem::Head { label: "Edit".into(), ts: Some("2026-11-03T10:00:00Z".into()) }));
        assert_eq!(items.last(), Some(&DiffItem::End));
    }

    #[test]
    fn labels_gaps_and_notes() {
        let h = |n| Hunk { old_start: n, new_start: n, lines: vec!["+x".into()] };
        let (items, _) = flatten_edits(&[edit(true, vec![], false), edit(false, vec![h(1), h(40)], true)]);
        let heads: Vec<_> = items.iter().filter_map(|i| match i { DiffItem::Head { label, .. } => Some(label.as_str()), _ => None }).collect();
        assert_eq!(heads, ["Created", "Edit 2 of 2"]);
        assert!(items.contains(&DiffItem::Note("An empty file.".into())));
        assert!(items.contains(&DiffItem::Gap));
        assert!(items.contains(&DiffItem::Note("Long edit: the rest isn’t shown.".into())));
        assert_eq!(flatten_edits(&[]), (vec![], true));
    }
}
