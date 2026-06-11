//! Spoken-list formatting: turn "one, buy milk. Two, walk the dog." into
//!
//! ```text
//! 1. Buy milk.
//! 2. Walk the dog.
//! ```
//!
//! Matching is deliberately conservative — a list only forms when the markers
//! run in sequence starting at 1, so "One day I went to the store" is never
//! mangled. Recognized markers at a sentence start:
//!
//! - cardinals with a comma/colon: "one, buy milk" (a bare space is ignored,
//!   so "two hours later" is safe)
//! - ordinals, comma or space: "first, do X" / "second do Y" / "finally do Z"
//! - digits: "1." / "2,"
//! - any of the above prefixed with "number"
//! - standalone markers as their own sentence ("One. Buy milk. Two. ...")

#[derive(Clone, Copy, PartialEq)]
enum Marker {
    /// A specific position; `allows_space` is true for strong markers
    /// (ordinals) that count even without a comma after them.
    Num { n: usize, allows_space: bool },
    /// "finally" / "lastly": matches whatever position comes next.
    Last,
}

const CARDINALS: [&str; 12] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
    "twelve",
];
const ORDINALS: [&str; 12] = [
    "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth",
    "eleventh", "twelfth",
];

fn marker_from_word(word: &str) -> Option<Marker> {
    if let Some(i) = CARDINALS.iter().position(|w| *w == word) {
        return Some(Marker::Num {
            n: i + 1,
            allows_space: false,
        });
    }
    if let Some(i) = ORDINALS.iter().position(|w| *w == word) {
        return Some(Marker::Num {
            n: i + 1,
            allows_space: true,
        });
    }
    match word {
        "firstly" => {
            return Some(Marker::Num {
                n: 1,
                allows_space: true,
            })
        }
        "secondly" => {
            return Some(Marker::Num {
                n: 2,
                allows_space: true,
            })
        }
        "thirdly" => {
            return Some(Marker::Num {
                n: 3,
                allows_space: true,
            })
        }
        "finally" | "lastly" => return Some(Marker::Last),
        _ => {}
    }
    if !word.is_empty() && word.chars().all(|c| c.is_ascii_digit()) {
        return Some(Marker::Num {
            n: word.parse().ok()?,
            allows_space: false,
        });
    }
    None
}

/// Parse a sentence as "marker [rest]". Returns the marker and the item text
/// after it, or None for the rest if the marker stands alone.
fn parse_marker(sentence: &str) -> Option<(Marker, Option<String>)> {
    let s = sentence.trim();
    let s = strip_prefix_ci(s, "number ").unwrap_or(s);
    let end = s.find([' ', ',', ':']).unwrap_or(s.len());
    let (word_raw, rest_raw) = s.split_at(end);
    let word = word_raw
        .trim_end_matches(['.', '!', '?'])
        .to_ascii_lowercase();
    let marker = marker_from_word(&word)?;
    let rest = rest_raw.trim_start_matches([',', ':']).trim();
    if rest.is_empty() {
        return Some((marker, None));
    }
    let allows_space = matches!(
        marker,
        Marker::Num {
            allows_space: true,
            ..
        } | Marker::Last
    );
    match rest_raw.chars().next() {
        Some(',') | Some(':') => Some((marker, Some(rest.to_string()))),
        Some(' ') if allows_space => Some((marker, Some(rest.to_string()))),
        _ => None,
    }
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Split on sentence-ending punctuation (keeping it attached).
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        cur.push(c);
        if ['.', '!', '?'].contains(&c) && chars.peek().is_none_or(|n| n.is_whitespace()) {
            let s = cur.trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            cur.clear();
        }
    }
    let s = cur.trim();
    if !s.is_empty() {
        out.push(s.to_string());
    }
    out
}

fn capitalized(text: &str) -> String {
    let mut flag = true;
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if flag && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            flag = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Try to collect a numbered list starting at `start`. Returns the formatted
/// item texts and how many sentences were consumed.
fn collect_list(sentences: &[String], start: usize) -> Option<(Vec<String>, usize)> {
    // A list must open with an explicit "1" marker.
    match parse_marker(&sentences[start])? {
        (Marker::Num { n: 1, .. }, _) => {}
        _ => return None,
    }

    let mut items: Vec<String> = Vec::new();
    let mut expected = 1;
    let mut j = start;
    while j < sentences.len() {
        let matches_expected = match parse_marker(&sentences[j]) {
            Some((Marker::Num { n, .. }, rest)) if n == expected => Some(rest),
            Some((Marker::Last, rest)) if expected > 1 => Some(rest),
            _ => None,
        };
        let Some(rest) = matches_expected else { break };
        j += 1;
        let mut item = rest.unwrap_or_default();
        // Sentences up to the next marker belong to the current item.
        while j < sentences.len() && parse_marker(&sentences[j]).is_none() {
            if !item.is_empty() {
                item.push(' ');
            }
            item.push_str(&sentences[j]);
            j += 1;
        }
        if item.is_empty() {
            break; // a dangling marker with no content isn't a list item
        }
        items.push(capitalized(&item));
        expected += 1;
    }

    if items.len() >= 2 {
        Some((items, j - start))
    } else {
        None
    }
}

/// Format any spoken numbered lists in `text` as numbered lines.
pub fn format_lists(text: &str) -> String {
    let sentences = split_sentences(text);
    let mut blocks: Vec<(String, bool)> = Vec::new(); // (text, is_list)
    let mut i = 0;
    while i < sentences.len() {
        if let Some((items, consumed)) = collect_list(&sentences, i) {
            let lines: Vec<String> = items
                .iter()
                .enumerate()
                .map(|(k, item)| format!("{}. {item}", k + 1))
                .collect();
            blocks.push((lines.join("\n"), true));
            i += consumed;
        } else {
            blocks.push((sentences[i].clone(), false));
            i += 1;
        }
    }

    let mut out = String::new();
    let mut prev_list = false;
    for (k, (block, is_list)) in blocks.iter().enumerate() {
        if k > 0 {
            out.push(if *is_list || prev_list { '\n' } else { ' ' });
        }
        out.push_str(block);
        prev_list = *is_list;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_cardinals_with_commas() {
        assert_eq!(
            format_lists("One, buy milk. Two, walk the dog. Three, call mom."),
            "1. Buy milk.\n2. Walk the dog.\n3. Call mom."
        );
    }

    #[test]
    fn standalone_markers() {
        assert_eq!(
            format_lists("One. Buy milk. Two. Walk the dog."),
            "1. Buy milk.\n2. Walk the dog."
        );
    }

    #[test]
    fn ordinals_with_spaces_and_finally() {
        assert_eq!(
            format_lists("First do the dishes. Second call the bank. Finally go to bed."),
            "1. Do the dishes.\n2. Call the bank.\n3. Go to bed."
        );
    }

    #[test]
    fn digits_reflow_onto_lines() {
        assert_eq!(
            format_lists("1. Buy milk. 2. Walk the dog."),
            "1. Buy milk.\n2. Walk the dog."
        );
    }

    #[test]
    fn number_prefix() {
        assert_eq!(
            format_lists("Number one, eat. Number two, sleep."),
            "1. Eat.\n2. Sleep."
        );
    }

    #[test]
    fn intro_text_is_preserved() {
        assert_eq!(
            format_lists("Here are my tasks. One, eat. Two, sleep."),
            "Here are my tasks.\n1. Eat.\n2. Sleep."
        );
    }

    #[test]
    fn no_false_positive_on_narrative_one() {
        let text = "One day I went to the store. It was raining.";
        assert_eq!(format_lists(text), text);
    }

    #[test]
    fn no_false_positive_without_a_second_item() {
        let text = "One, single item only.";
        assert_eq!(format_lists(text), text);
    }

    #[test]
    fn cardinal_with_space_is_not_a_marker() {
        let text = "Two hours later we left. It was late.";
        assert_eq!(format_lists(text), text);
    }

    #[test]
    fn multi_sentence_items_stay_together() {
        assert_eq!(
            format_lists("One, eat lunch. It was good. Two, sleep."),
            "1. Eat lunch. It was good.\n2. Sleep."
        );
    }
}
