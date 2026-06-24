// ext/thapthim/src/normalize.rs
//
// Thai text normalization, ported 1:1 from the Ruby `Thapthim::std_normalize`
// (lib/thapthim/normalize_std.rb, itself modified from PyThaiNLP) so the Ruby and Python bindings
// share one implementation. The `normalize:` option in both languages routes here.
//
// fancy-regex (not the `regex` crate) is used because `remove_spaces_before_marks` needs a negative
// lookbehind, which `regex` cannot express. Patterns are compiled once into a process-global
// `Normalizer`.
//
// Parity note: the per-character "collapse repeats" pass mirrors the Ruby quirk that
// `NOREPEAT_CHARS.each_char` walks the *literal* class strings — so a range fragment like
// "เ-ไ" contributes the three chars เ, '-', ไ (NOT the expanded range). We build the
// same literal string and iterate its chars to stay byte-identical with Ruby.

use std::sync::OnceLock;
use fancy_regex::Regex;

// Class-definition fragments, identical to the Ruby constants. Used two ways: spliced inside
// `[...]` (where ranges like `A-B` expand) AND, for NOREPEAT, walked char-by-char (where `A-B`
// is three literal chars). Both uses must see the exact same source string.
const CONS: &str = "\u{0E01}-\u{0E23}\u{0E25}\u{0E27}-\u{0E2E}";
const ABOVE: &str = "\u{0E31}\u{0E34}-\u{0E37}\u{0E4D}\u{0E47}";
const BELOW: &str = "\u{0E38}\u{0E39}";
const LEAD: &str = "\u{0E40}-\u{0E44}";
const FOLLOW: &str = "\u{0E30}\u{0E32}\u{0E33}";
const TONES: &str = "\u{0E48}-\u{0E4B}";
const EXTRA_MARKS: &str = "\u{0E3A}\u{0E4C}\u{0E4D}\u{0E4E}";
// Standalone chars referenced inside format! patterns (can't write \u{..} there — braces clash
// with format placeholders).
const THANTHAKHAT: &str = "\u{0E4C}"; // ์
const NIKHAHIT: &str = "\u{0E4D}"; // ํ
const SARA_AA: &str = "\u{0E32}"; // า

struct Normalizer {
    zw: Regex,
    dup_newlines: Regex,
    dup_spaces: Regex,
    spaces_before_marks: Regex,
    reorder: Vec<(Regex, String)>,
    norepeat: Vec<(Regex, String)>,
    tone_runs: Regex,
    dangling_lead: Regex,
    dangling_after_space: Regex,
}

fn re(pat: &str) -> Regex {
    Regex::new(pat).expect("static normalize pattern compiles")
}

impl Normalizer {
    fn new() -> Self {
        let vowels = format!("{ABOVE}{BELOW}{LEAD}{FOLLOW}\u{0E24}\u{0E26}");
        let dangling = format!("{ABOVE}{BELOW}{TONES}{EXTRA_MARKS}");

        // REORDER_PAIRS, in order (normalize_std.rb).
        let reorder = vec![
            (re("\u{0E40}\u{0E40}"), "\u{0E41}".to_string()), // เ + เ = แ
            // swap tone/thanthakhat with above/below vowels
            (re(&format!("([{TONES}{THANTHAKHAT}]+)([{ABOVE}{BELOW}]+)")), "${2}${1}".to_string()),
            // ํ + tone* + า = tone* + ำ
            (re(&format!("{NIKHAHIT}([{TONES}]*){SARA_AA}")), "${1}\u{0E33}".to_string()),
            // swap follow-vowels with tones
            (re(&format!("([{FOLLOW}]+)([{TONES}]+)")), "${2}${1}".to_string()),
            // ๅ -> า unless preceded by ฤ/ฦ
            (re("([^\u{0E24}\u{0E26}])\u{0E45}"), "${1}\u{0E32}".to_string()),
        ];

        // NOREPEAT_CHARS = FOLLOW + LEAD + ABOVE + BELOW + EXTRA, walked literally (ranges NOT
        // expanded), exactly like Ruby's each_char. Collapse `(c[ ]*)+c` -> c for each char c.
        let norepeat_src = format!("{FOLLOW}{LEAD}{ABOVE}{BELOW}{EXTRA_MARKS}");
        let norepeat = norepeat_src
            .chars()
            .map(|c| {
                let esc = regex::escape(&c.to_string());
                (re(&format!("({esc}[ ]*)+{esc}")), c.to_string())
            })
            .collect();

        Normalizer {
            zw: re("[\u{200B}\u{200C}]"),
            dup_newlines: re(r"[ \n]*\n[ \n]*"),
            dup_spaces: re(" +"),
            spaces_before_marks: re(&format!("([{CONS}])(?<![{vowels}][{CONS}]) ([{dangling}])")),
            reorder,
            norepeat,
            tone_runs: re(&format!("[{TONES}]+")),
            dangling_lead: re(&format!("^[{dangling}]+")),
            dangling_after_space: re(&format!(" +[{dangling}]+")),
        }
    }

    fn normalize(&self, input: &str) -> String {
        if input.is_empty() {
            return String::new();
        }
        // remove_zw
        let mut text = self.zw.replace_all(input, "").into_owned();
        // remove_dup_spaces: collapse newlines, then spaces, then strip
        text = self.dup_newlines.replace_all(&text, "\n").into_owned();
        text = self.dup_spaces.replace_all(&text, " ").into_owned();
        text = text.trim().to_string();
        // remove_spaces_before_marks
        text = self.spaces_before_marks.replace_all(&text, "${1}${2}").into_owned();
        // remove_repeat_vowels: reorder, collapse per-char repeats, collapse tone runs to last tone
        for (pat, rep) in &self.reorder {
            text = pat.replace_all(&text, rep.as_str()).into_owned();
        }
        for (pat, rep) in &self.norepeat {
            text = pat.replace_all(&text, rep.as_str()).into_owned();
        }
        text = self
            .tone_runs
            .replace_all(&text, |caps: &fancy_regex::Captures| {
                caps[0].chars().last().unwrap().to_string()
            })
            .into_owned();
        // remove_dangling: strip a leading run, then any run following spaces
        text = self.dangling_lead.replace(&text, "").into_owned();
        text = self.dangling_after_space.replace_all(&text, " ").into_owned();
        text
    }
}

fn normalizer() -> &'static Normalizer {
    static N: OnceLock<Normalizer> = OnceLock::new();
    N.get_or_init(Normalizer::new)
}

/// Normalize Thai text: strip zero-width chars, collapse duplicate spaces/marks, reorder
/// misordered vowel/tone sequences, and drop dangling combining marks. Mirrors the Ruby
/// `Thapthim::std_normalize`.
pub fn std_normalize(text: &str) -> String {
    normalizer().normalize(text)
}
