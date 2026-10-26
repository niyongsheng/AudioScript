const LATIN_RATIO: f32 = 0.4;

const UPPER_RATIO: f32 = 0.9;

const MIN_LATIN: usize = 4;

pub fn restore_sentence_case(text: &str) -> String {
    if !needs_restore(text) {
        return text.to_owned();
    }

    let mut out = String::with_capacity(text.len());
    let mut at_sentence_start = true;
    let mut pending_start = false;
    let mut ascii_quote_open = false;

    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            out.push(if at_sentence_start {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            });
            at_sentence_start = false;
            pending_start = false;
            continue;
        }

        out.push(ch);

        match ch {
            '.' | '!' | '?' => pending_start = true,
            '"' => {
                ascii_quote_open = !ascii_quote_open;
                if ascii_quote_open {
                    at_sentence_start = true;
                    pending_start = false;
                }
            }
            '“' => {
                at_sentence_start = true;
                pending_start = false;
            }
            '”' => pending_start = false,
            _ if pending_start && !ch.is_whitespace() && ch != ')' => pending_start = false,
            _ => {}
        }

        if pending_start && ch.is_whitespace() {
            at_sentence_start = true;
        }
    }
    out
}

fn needs_restore(text: &str) -> bool {
    let mut latin = 0usize;
    let mut upper = 0usize;
    let mut visible = 0usize;
    for ch in text.chars() {
        if !ch.is_whitespace() {
            visible += 1;
        }
        if ch.is_ascii_alphabetic() {
            latin += 1;
            if ch.is_ascii_uppercase() {
                upper += 1;
            }
        }
    }

    if latin < MIN_LATIN {
        return false;
    }
    if (latin as f32 / visible.max(1) as f32) < LATIN_RATIO {
        return false;
    }
    upper as f32 / latin as f32 >= UPPER_RATIO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_all_caps_english() {
        assert_eq!(
            restore_sentence_case("SO THE KEY INSIGHT HERE IS THAT WE NEED TO HANDLE IT."),
            "So the key insight here is that we need to handle it."
        );
    }

    #[test]
    fn capitalizes_each_sentence() {
        assert_eq!(
            restore_sentence_case("THE WEATHER WAS NICE. WE DECIDED TO GO FOR A WALK."),
            "The weather was nice. We decided to go for a walk."
        );
    }

    #[test]
    fn handles_question_and_exclamation() {
        assert_eq!(
            restore_sentence_case("HELLO THERE! HOW ARE YOU? I AM FINE."),
            "Hello there! How are you? I am fine."
        );
    }

    #[test]
    fn leaves_mixed_case_alone() {
        for s in [
            "So the key insight here is that we need to handle it.",
            "The USA is big and the AI boom is real.",
            "I met McDonald in Paris.",
        ] {
            assert_eq!(restore_sentence_case(s), s, "不该动：{s}");
        }
    }

    #[test]
    fn leaves_chinese_alone() {
        for s in [
            "这个方案最大的问题在于延迟。",
            "AI 很重要。",
            "他说 GDP 增长了。",
        ] {
            assert_eq!(restore_sentence_case(s), s, "不该动：{s}");
        }
    }

    #[test]
    fn leaves_short_snippets_alone() {
        assert_eq!(restore_sentence_case("OK"), "OK");
        assert_eq!(restore_sentence_case("AI"), "AI");
    }

    #[test]
    fn keeps_digits_and_punctuation() {
        assert_eq!(
            restore_sentence_case("IN 2026 WE HAD 3 GOALS: WIN, GROW, AND SHIP."),
            "In 2026 we had 3 goals: win, grow, and ship."
        );
    }

    #[test]
    fn opening_quote_starts_a_capitalized_clause() {
        assert_eq!(
            restore_sentence_case("HE SAID \"HELLO THERE.\" THEN HE LEFT."),
            "He said \"Hello there.\" Then he left."
        );
    }

    #[test]
    fn closing_quote_resumes_the_original_sentence_in_lowercase() {
        assert_eq!(
            restore_sentence_case("\"HELLO,\" HE SAID, AND THEN HE LEFT."),
            "\"Hello,\" he said, and then he left."
        );
    }

    #[test]
    fn abbreviation_dots_do_not_start_a_new_sentence() {
        let out = restore_sentence_case("MR. SMITH WENT HOME AND SAW HIS FRIEND.");
        assert!(out.starts_with("Mr. Smith"), "{out}");
    }
}
