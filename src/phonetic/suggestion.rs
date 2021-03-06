// Suggestion making module.

use std::cmp::Ordering;
use edit_distance::edit_distance;
use rupantor::avro::AvroPhonetic;
use rustc_hash::FxHashMap;

use crate::phonetic::database::Database;
use crate::utility::Utility;

pub(crate) struct PhoneticSuggestion {
    suggestions: Vec<String>,
    database: Database,
    // Cache for storing dictionary searches.
    cache: FxHashMap<String, Vec<String>>,
    phonetic: AvroPhonetic,
}

impl PhoneticSuggestion {
    pub(crate) fn new() -> Self {
        PhoneticSuggestion {
            suggestions: Vec::new(),
            database: Database::new(),
            cache: FxHashMap::default(),
            phonetic: AvroPhonetic::new(),
        }
    }

    /// Add suffix(গুলো, মালা...etc) to the dictionary suggestions and return them.
    /// This function gets the suggestion list from the stored cache.
    fn add_suffix_to_suggestions(&self, splitted: &(String, String, String)) -> Vec<String> {
        let middle = &splitted.1;
        let mut list = Vec::new();
        if middle.len() > 2 {
            for i in 1..middle.len() {
                let suffix_key = &middle[i..];
                if let Some(suffix) = self.database.find_suffix(suffix_key) {
                    let key = &middle[0..(middle.len() - suffix_key.len())];
                    if self.cache.contains_key(key) {
                        for item in &self.cache[key] {
                            let item_rmc = item.chars().last().unwrap(); // Right most character.
                            let suffix_lmc = suffix.chars().nth(0).unwrap(); // Left most character.
                            if item_rmc.is_vowel() && suffix_lmc.is_kar() {
                                let word = format!("{}{}{}", item, "\u{09DF}", suffix);
                                list.push(word);
                            } else {
                                if item_rmc == '\u{09CE}' {
                                    // Khandatta
                                    let word = format!(
                                        "{}{}{}",
                                        item.trim_end_matches('\u{09CE}'),
                                        "\u{09A4}",
                                        suffix
                                    );
                                    list.push(word);
                                } else if item_rmc == '\u{0982}' {
                                    // Anushar
                                    let word = format!(
                                        "{}{}{}",
                                        item.trim_end_matches('\u{0982}'),
                                        "\u{0999}",
                                        suffix
                                    );
                                    list.push(word);
                                } else {
                                    let word = format!("{}{}", item, suffix);
                                    list.push(word);
                                }
                            }
                        }
                    }
                }
            }
        }

        if !list.is_empty() {
            list
        } else {
            self.cache
                .get(&splitted.1)
                .cloned()
                .unwrap_or_else(|| Vec::new())
        }
    }

    /// Make suggestions from the given `term`.
    pub(crate) fn suggest(&mut self, term: &str) -> Vec<String> {
        let mut suggestions: Vec<String> = Vec::new();
        let splitted_string = split_string(term);

        let phonetic = self.phonetic.convert(&splitted_string.1);

        if !self.cache.contains_key(&splitted_string.1) {
            let mut dictionary = self.database.search_dictionary(&splitted_string.1);
            // Auto Correct
            if let Some(corrected) = self.database.get_corrected(&splitted_string.1) {
                let word = self.phonetic.convert(&corrected);
                suggestions.push(word.clone());
                // Add it to the cache for adding suffix later.
                dictionary.push(word);
            }
            // Cache it.
            self.cache.insert(splitted_string.1.clone(), dictionary);
        }

        let mut suggestions_with_suffix = self.add_suffix_to_suggestions(&splitted_string);

        suggestions_with_suffix.sort_by(|a, b| {
            let dist1 = edit_distance(&phonetic, a);
            let dist2 = edit_distance(&phonetic, b);

            if dist1 < dist2 {
                Ordering::Less
            } else if dist1 > dist2 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        suggestions.append(&mut suggestions_with_suffix);

        // Last Item: Phonetic. Check if it already contains.
        if !suggestions.contains(&phonetic) {
            suggestions.push(phonetic);
        }

        for item in suggestions.iter_mut() {
            *item = format!("{}{}{}", splitted_string.0, item, splitted_string.2);
        }

        // Emoticons Auto Corrects
        if let Some(emoticon) = self.database.get_corrected(term) {
            suggestions.insert(0, emoticon);
        }

        suggestions
    }
}

// Implement Default trait on PhoneticSuggestion, actually for testing convenience.
impl Default for PhoneticSuggestion {
    fn default() -> Self {
        PhoneticSuggestion::new()
    }
}

/// Split the string into three parts.
/// This function splits preceding and trailing meta characters.
fn split_string(input: &str) -> (String, String, String) {
    let meta = "-]~!@#%&*()_=+[{}'\";<>/?|.,";
    let mut first_index = 0;
    let mut last_index = 0;
    let mut encountered_alpha = false;

    for (index, c) in input.chars().enumerate() {
        if !meta.contains(c) {
            first_index = index;
            encountered_alpha = true;
            break;
        }
    }

    // Corner case: If we haven't yet encountered an alpha or
    // a numeric character, then the string has no middle part
    // or last part we need. So return "" for them ;)
    if !encountered_alpha {
        return (input[..].to_owned(), "".to_owned(), "".to_owned());
    }

    for (index, c) in input.chars().rev().enumerate() {
        if !meta.contains(c) {
            last_index = input.len() - index - 1;
            break;
        }
    }

    let first_part = input[0..first_index].to_owned();
    let middle_part = input[first_index..=last_index].to_owned();
    let last_part = input[last_index + 1..].to_owned();

    (first_part, middle_part, last_part)
}

#[cfg(test)]
mod tests {
    use super::split_string;
    use super::PhoneticSuggestion;
    use rustc_hash::FxHashMap;

    #[test]
    fn test_emoticon() {
        let mut suggestion = PhoneticSuggestion::new();

        assert_eq!(suggestion.suggest(":)"), vec![":)", "ঃ)"]);
    }

    #[test]
    fn test_suggestion() {
        let mut suggestion = PhoneticSuggestion::new();

        assert_eq!(
            suggestion.suggest("a"),
            vec![
                "আ",
                "আঃ",
                "া",
                "এ",
                "অ্যা",
                "অ্যাঁ"
            ]
        );
        assert_eq!(
            suggestion.suggest("as"),
            vec!["আস", "আশ", "এস", "আঁশ"]
        );
        assert_eq!(
            suggestion.suggest("asgulo"),
            vec![
                "আসগুলো",
                "আশগুলো",
                "এসগুলো",
                "আঁশগুলো",
                "আসগুল"
            ]
        );
        assert_eq!(
            suggestion.suggest("(as)"),
            vec!["(আস)", "(আশ)", "(এস)", "(আঁশ)"]
        );
    }

    #[test]
    fn test_suffix() {
        let mut cache: FxHashMap<String, Vec<String>> = FxHashMap::default();
        cache.insert(
            "computer".to_string(),
            vec!["কম্পিউটার".to_string()],
        );
        cache.insert("ebong".to_string(), vec!["এবং".to_string()]);

        let suggestion = PhoneticSuggestion {
            cache,
            ..Default::default()
        };

        assert_eq!(
            suggestion.add_suffix_to_suggestions(&(
                "".to_string(),
                "computer".to_string(),
                "".to_string()
            )),
            vec!["কম্পিউটার"]
        );
        assert_eq!(
            suggestion.add_suffix_to_suggestions(&(
                "".to_string(),
                "computere".to_string(),
                "".to_string()
            )),
            vec!["কম্পিউটারে"]
        );
        assert_eq!(
            suggestion.add_suffix_to_suggestions(&(
                "".to_string(),
                "computergulo".to_string(),
                "".to_string()
            )),
            vec!["কম্পিউটারগুলো"]
        );
        assert_eq!(
            suggestion.add_suffix_to_suggestions(&(
                "".to_string(),
                "ebongmala".to_string(),
                "".to_string()
            )),
            vec!["এবঙমালা"]
        );
    }

    #[test]
    fn test_split_string() {
        assert_eq!(
            split_string("[][][][]"),
            ("[][][][]".to_owned(), "".to_owned(), "".to_owned())
        );
        assert_eq!(
            split_string("t*"),
            ("".to_owned(), "t".to_owned(), "*".to_owned())
        );
        assert_eq!(
            split_string("1"),
            ("".to_owned(), "1".to_owned(), "".to_owned())
        );
        assert_eq!(
            split_string("#\"percent%sign\"#"),
            (
                "#\"".to_owned(),
                "percent%sign".to_owned(),
                "\"#".to_owned()
            )
        );
        assert_eq!(
            split_string("text"),
            ("".to_owned(), "text".to_owned(), "".to_owned())
        );
        assert_eq!(
            split_string(":)"),
            ("".to_owned(), ":".to_owned(), ")".to_owned())
        );
    }
}
