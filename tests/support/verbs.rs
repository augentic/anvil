//! Live verbs
//!
//! Reads the verb names out of `emery --help`, so a suite that needs to know
//! which commands exist learns it from the shipped surface rather than from a
//! list that would have to be kept in step by hand.

/// Sorted live verb names from the `Commands:` section of `emery --help`.
pub fn verbs(help: &str) -> Vec<String> {
    let mut names: Vec<String> = help
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            let kebab = !name.is_empty()
                && !name.starts_with('-')
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
            kebab.then(|| name.to_owned())
        })
        .collect();
    names.sort_unstable();
    names
}
