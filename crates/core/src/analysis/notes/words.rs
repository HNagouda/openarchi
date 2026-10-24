//! Months and lists written out as words, for the design notes.

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn month_name(month: u32) -> &'static str {
    MONTH_NAMES[(month as usize + 11) % 12]
}

/// [5, 6, 7, 8, 9] -> "May to September", [12, 1, 2] -> "December to
/// February", [1, 4] -> "January and April".
fn month_span(months: &[u32]) -> String {
    let is_run = months.windows(2).all(|pair| pair[1] == pair[0] % 12 + 1);
    // A run across the new year comes in calendar order: [1, 2, 12].
    let wraps = !is_run && months.len() > 2 && months.contains(&12) && months.contains(&1);
    let ordered: Vec<u32> = if wraps {
        let start = months
            .iter()
            .position(|&month| month > months[0] + 1)
            .unwrap_or(0);
        months[start..]
            .iter()
            .chain(&months[..start])
            .copied()
            .collect()
    } else {
        months.to_vec()
    };
    let is_ordered_run = ordered.windows(2).all(|pair| pair[1] == pair[0] % 12 + 1);

    let names: Vec<&str> = ordered.iter().map(|&month| month_name(month)).collect();
    match names.as_slice() {
        [] => String::new(),
        [only] => only.to_string(),
        [first, .., last] if is_ordered_run && names.len() > 2 => format!("{first} to {last}"),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// ["a", "b", "c"] -> "a, b or c".
pub fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// A run of months as "from May to September", anything else as "in January and April".
pub fn during(months: &[u32]) -> String {
    let span = month_span(months);
    if span.contains(" to ") {
        format!("from {span}")
    } else {
        format!("in {span}")
    }
}

pub fn capitalised(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_runs_read_as_a_span_and_gaps_as_a_list() {
        assert_eq!(month_span(&[5, 6, 7, 8, 9]), "May to September");
        assert_eq!(month_span(&[1, 2, 12]), "December to February");
        assert_eq!(month_span(&[1, 4]), "January and April");
        assert_eq!(month_span(&[1, 2]), "January and February");
        assert_eq!(month_span(&[7]), "July");
    }
}
