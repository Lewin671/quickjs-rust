pub(super) fn is_non_capturing_group(pattern: &[char], pc: usize) -> bool {
    pattern.get(pc + 1) == Some(&'?') && pattern.get(pc + 2) == Some(&':')
}

pub(super) fn closing_group(pattern: &[char], pc: usize) -> Option<usize> {
    let mut escaped = false;
    let mut in_class = false;
    let mut depth = 0usize;
    for (offset, char) in pattern[pc + 1..].iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == '[' {
            in_class = true;
        } else if *char == ']' {
            in_class = false;
        } else if !in_class && *char == '(' {
            depth += 1;
        } else if !in_class && *char == ')' && depth > 0 {
            depth -= 1;
        } else if !in_class && *char == ')' {
            return Some(pc + 1 + offset);
        }
    }
    None
}

pub(super) fn group_alternatives(
    pattern: &[char],
    start_pc: usize,
    end_pc: usize,
) -> Vec<(usize, usize)> {
    let mut alternatives = Vec::new();
    let mut start = start_pc;
    let mut escaped = false;
    let mut in_class = false;
    let mut depth = 0usize;
    for (index, char) in pattern
        .iter()
        .enumerate()
        .take(end_pc)
        .skip(start_pc)
        .map(|(index, char)| (index, *char))
    {
        if escaped {
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '[' {
            in_class = true;
        } else if char == ']' {
            in_class = false;
        } else if !in_class && char == '(' {
            depth += 1;
        } else if !in_class && char == ')' && depth > 0 {
            depth -= 1;
        } else if !in_class && char == '|' && depth == 0 {
            alternatives.push((start, index));
            start = index + 1;
        }
    }
    alternatives.push((start, end_pc));
    alternatives
}
