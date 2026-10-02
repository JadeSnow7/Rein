//! Full-file review and a unified diff computed locally, matching Python's `difflib` output.

use std::collections::HashMap;
use std::io::IsTerminal;

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const RESET: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Auto,
    Always,
    Never,
}

impl Color {
    pub fn parse(text: &str) -> Option<Color> {
        match text {
            "auto" => Some(Color::Auto),
            "always" => Some(Color::Always),
            "never" => Some(Color::Never),
            _ => None,
        }
    }
}

/// Split after "\n", "\r\n" or "\r", keeping the line endings (like `str.splitlines(True)`).
pub fn split_lines_keep_ends(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let (mut lines, mut start, mut index) = (Vec::new(), 0, 0);
    while index < bytes.len() {
        let end = match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => Some(index + 2),
            b'\r' | b'\n' => Some(index + 1),
            _ => None,
        };
        match end {
            Some(end) => {
                lines.push(&text[start..end]);
                start = end;
                index = end;
            }
            None => index += 1,
        }
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

fn strip_ending(line: &str) -> &str {
    line.strip_suffix("\r\n").or_else(|| line.strip_suffix('\n')).or_else(|| line.strip_suffix('\r')).unwrap_or(line)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag {
    Equal,
    Replace,
    Delete,
    Insert,
}

type Opcode = (Tag, usize, usize, usize, usize);

/// Longest matching block inside a[alo..ahi] and b[blo..bhi]; ties keep the earliest match.
fn longest_match(
    a: &[&str],
    b2j: &HashMap<&str, Vec<usize>>,
    alo: usize,
    ahi: usize,
    blo: usize,
    bhi: usize,
) -> (usize, usize, usize) {
    let (mut best_i, mut best_j, mut best_size) = (alo, blo, 0);
    let mut j2len: HashMap<usize, usize> = HashMap::new();
    for (i, line) in a.iter().enumerate().take(ahi).skip(alo) {
        let mut next: HashMap<usize, usize> = HashMap::new();
        for &j in b2j.get(line).map(Vec::as_slice).unwrap_or_default() {
            if j < blo {
                continue;
            }
            if j >= bhi {
                break;
            }
            let k = if j > 0 { j2len.get(&(j - 1)).copied().unwrap_or(0) } else { 0 } + 1;
            next.insert(j, k);
            if k > best_size {
                (best_i, best_j, best_size) = (i + 1 - k, j + 1 - k, k);
            }
        }
        j2len = next;
    }
    (best_i, best_j, best_size)
}

fn opcodes(a: &[&str], b: &[&str]) -> Vec<Opcode> {
    let mut b2j: HashMap<&str, Vec<usize>> = HashMap::new();
    for (j, line) in b.iter().enumerate() {
        b2j.entry(line).or_default().push(j);
    }
    let mut blocks = Vec::new();
    let mut queue = vec![(0, a.len(), 0, b.len())];
    while let Some((alo, ahi, blo, bhi)) = queue.pop() {
        let (i, j, k) = longest_match(a, &b2j, alo, ahi, blo, bhi);
        if k > 0 {
            blocks.push((i, j, k));
            if alo < i && blo < j {
                queue.push((alo, i, blo, j));
            }
            if i + k < ahi && j + k < bhi {
                queue.push((i + k, ahi, j + k, bhi));
            }
        }
    }
    blocks.sort_unstable();
    let mut merged: Vec<(usize, usize, usize)> = Vec::new();
    for (i, j, k) in blocks {
        match merged.last_mut() {
            Some(last) if last.0 + last.2 == i && last.1 + last.2 == j => last.2 += k,
            _ => merged.push((i, j, k)),
        }
    }
    merged.push((a.len(), b.len(), 0));
    let (mut i, mut j, mut codes) = (0, 0, Vec::new());
    for (ai, bj, size) in merged {
        let tag = match (i < ai, j < bj) {
            (true, true) => Some(Tag::Replace),
            (true, false) => Some(Tag::Delete),
            (false, true) => Some(Tag::Insert),
            (false, false) => None,
        };
        if let Some(tag) = tag {
            codes.push((tag, i, ai, j, bj));
        }
        (i, j) = (ai + size, bj + size);
        if size > 0 {
            codes.push((Tag::Equal, ai, i, bj, j));
        }
    }
    codes
}

fn grouped(mut codes: Vec<Opcode>, n: usize) -> Vec<Vec<Opcode>> {
    if codes.is_empty() {
        codes.push((Tag::Equal, 0, 1, 0, 1));
    }
    if let Some(first) = codes.first_mut() {
        if first.0 == Tag::Equal {
            *first = (
                Tag::Equal,
                first.1.max(first.2.saturating_sub(n)),
                first.2,
                first.3.max(first.4.saturating_sub(n)),
                first.4,
            );
        }
    }
    if let Some(last) = codes.last_mut() {
        if last.0 == Tag::Equal {
            *last = (Tag::Equal, last.1, last.2.min(last.1 + n), last.3, last.4.min(last.3 + n));
        }
    }
    let (mut groups, mut group) = (Vec::new(), Vec::new());
    for (tag, mut i1, i2, mut j1, j2) in codes {
        if tag == Tag::Equal && i2 - i1 > 2 * n {
            group.push((tag, i1, i2.min(i1 + n), j1, j2.min(j1 + n)));
            groups.push(std::mem::take(&mut group));
            i1 = i1.max(i2.saturating_sub(n));
            j1 = j1.max(j2.saturating_sub(n));
        }
        group.push((tag, i1, i2, j1, j2));
    }
    let only_context = group.len() == 1 && group[0].0 == Tag::Equal;
    if !group.is_empty() && !only_context {
        groups.push(group);
    }
    groups
}

fn range(start: usize, stop: usize) -> String {
    match stop - start {
        1 => format!("{}", start + 1),
        0 => format!("{start},0"),
        length => format!("{},{length}", start + 1),
    }
}

/// Unified diff with three lines of context, one entry per output line.
pub fn unified_diff(before: &str, after: &str, from_name: &str, to_name: &str) -> Vec<String> {
    let (a, b) = (split_lines_keep_ends(before), split_lines_keep_ends(after));
    let mut out = Vec::new();
    for group in grouped(opcodes(&a, &b), 3) {
        if out.is_empty() {
            out.push(format!("--- {from_name}\n"));
            out.push(format!("+++ {to_name}\n"));
        }
        let (first, last) = (group[0], group[group.len() - 1]);
        out.push(format!("@@ -{} +{} @@\n", range(first.1, last.2), range(first.3, last.4)));
        for (tag, i1, i2, j1, j2) in group {
            if tag == Tag::Equal {
                out.extend(a[i1..i2].iter().map(|line| format!(" {line}")));
                continue;
            }
            if matches!(tag, Tag::Replace | Tag::Delete) {
                out.extend(a[i1..i2].iter().map(|line| format!("-{line}")));
            }
            if matches!(tag, Tag::Replace | Tag::Insert) {
                out.extend(b[j1..j2].iter().map(|line| format!("+{line}")));
            }
        }
    }
    out
}

fn numbered(text: &str) -> Vec<String> {
    split_lines_keep_ends(text)
        .iter()
        .enumerate()
        .map(|(index, line)| format!("{:>4} | {}", index + 1, strip_ending(line)))
        .collect()
}

pub fn render_review(original: &str, candidate: &str, color: Color) -> String {
    let mut lines = vec!["=== original file ===".to_string()];
    lines.extend(numbered(original));
    lines.push("=== candidate file ===".to_string());
    lines.extend(numbered(candidate));
    lines.push("=== diff ===".to_string());
    let use_color = match color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => std::io::stdout().is_terminal(),
    };
    let diff: String = unified_diff(original, candidate, "hello.cpp", "hello.cpp (candidate)")
        .into_iter()
        .map(|line| {
            let shade = if !use_color {
                ""
            } else if line.starts_with('-') {
                RED
            } else if line.starts_with('+') {
                GREEN
            } else {
                ""
            };
            if shade.is_empty() {
                line
            } else {
                format!("{shade}{line}{RESET}")
            }
        })
        .collect();
    lines.join("\n") + "\n" + &diff
}
