// SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
// SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
// SPDX-FileType: SOURCE
// SPDX-License-Identifier: Apache-2.0
//
// The Thai Character Cluster (TCC) grammar built in `TccSegmenter::new` (the `raw_rules` set) is
// ported from PyThaiNLP's `pythainlp/tokenize/tcc.py`, implementing the TCC rules proposed by
// Theeramunkong et al. (2000) <https://doi.org/10.1145/355214.355225>.
// Credits:
//   * TCC: Jakkrit TeCho
//   * Grammar: Wittawat Jitkrittum (jtcc, https://github.com/wittawatj/jtcc/blob/master/TCC.g)
// The western/Thai-numeral grouping rules and the `[\s\S]` full-coverage fallback are Thapthim
// additions, not part of the upstream grammar.
//
// ext/thapthim/src/tcc.rs
use regex::Regex;

pub struct TccSegmenter {
    tcc_regex: Regex,
}

impl Default for TccSegmenter {
    fn default() -> Self {
        Self::new()
    }
}

impl TccSegmenter {
    pub fn new() -> Self {
        let c = "[ก-ฮ]";
        let x = "[ก-ฮ]";
        let t = "[่-๋]?";
        let k = "(XX?(ุ|ู|ิ)?[์])?".replace("X", x);

        let raw_rules = vec![
            "c[ั]([่-๋]c)?", "c[ั]([่-๋]c)?k", "เc็ck", "เcctาะk", "เccีtยะk",
            "เccีtย", "เc[ิีุู]tย", "เcc็ck", "เcิc์ck", "เcิtck", "เcีtยะ?", 
            "เcืtอะk", "เcื", "เctา?ะ?", "c[ึื]tck", "c[ะ-ู]tk", "c[ิุู]์", 
            "cรรc์", "c็", "ct[ะาำ]?", "แc็ck", "แcc์k", "แctะk", "แcc็ck", 
            "แccc์k", "โctะk", "[เ-ไ]ctk", "ก็", "อึ", "หึ",
        ];

        let compiled_rules: Vec<String> = raw_rules
            .iter()
            .map(|rule| rule.replace("c", c).replace("t", t).replace("k", &k))
            .collect();

        // `w` is one "foreign word" character: any Unicode letter or combining mark, plus the
        // ASCII digits/underscore, MINUS the Thai script (`--\p{Thai}`, which Thai TCC rules own).
        // It broadens the old ASCII-only `[A-Za-z0-9_]` so a contiguous non-Thai run groups as ONE
        // cluster instead of shredding to single chars: diacritic Latin (dòufu, tōfu — the diacritic
        // is a precomposed `\p{L}` or a decomposed `\p{M}`), CJK (豆腐), Hangul (두부), and any other
        // non-Thai script. ASCII digits 0-9 are kept explicitly (digits are `\p{Nd}`, not `\p{L}`);
        // Thai digits stay excluded so rule 3 below still owns them.
        let w = r"[\p{L}\p{M}0-9_--\p{Thai}]";
        // The western/foreign-token rule, built from `w` (see rule 2 below).
        let western = format!(r"[@#]?{w}+(?:[.,@:/#-]+{w}+)*");

        // Two protective rules, placed first so they win over the `.` single-char fallback:
        //  1. `<[^<>]*>` keeps an angle-bracket tag (e.g. <NE>, </NE>, <Hello>) as ONE cluster.
        //  2. `[@#]?w+(?:[.,@:/#-]+w+)*` keeps a contiguous non-Thai (Latin/digit/CJK/…) run
        //     as ONE cluster, matching ssg's western-token convention. The connector punctuation
        //     (`. , @ : / # -`) is ANCHORED — it may only appear *between* two alphanumeric runs
        //     (3.5, URLs, ranges, a@b.com), never leading or trailing, with `@`/`#` the sole
        //     exception that may lead (@handles, #tags). The anchoring matters next to Thai script:
        //     the old unanchored class let a separator lead, so a date like `พ.ศ.2568` had its
        //     abbreviation period stolen forward into a `.2568` token (and `พ.ศ.` could never be
        //     reassembled by the dictionary). Freeing that period lets it bind back: `พ.ศ.`|`2568`.
        //     Both the word and syllable decodes build on this shared grid, so it groups
        //     consistently for both (modern text gain; see below).
        // (Mirrors the training-time western/markup token protection.)
        // Fallback is `[\s\S]` (every character, incl. newlines/control), not `.` — plain `.`
        // skips `\n`, leaving a gap in the grid so the newline gets glued to its neighbour.
        // This guarantees the TCC grid covers every byte of arbitrary input.
        //  3. `[๐-๙]+(?:[.,:][๐-๙]+)*` keeps a run of THAI numerals — including interior
        //     decimal/thousands/time separators (๒๕๖๙, ๑๔.๓๐, ๐๘๑๒๓๔๕๖๗๘) — as ONE cluster,
        //     mirroring rule 2 for ASCII digits. Without it the bare Thai digits fall through
        //     to the single-char fallback and the lattice shreds years/dates/phone strings.
        //     The interior-separator join matches the trusted modern convention: LST20 joins
        //     ASCII `digit.digit` 614:9 (the analogous case; LST20/VISTEC have no Thai-numeral
        //     separator examples). Only BEST's older convention splits these — not followed.
        let master_pattern = format!(
            r"<[^<>]*>|{western}|[๐-๙]+(?:[.,:][๐-๙]+)*|{}|[\s\S]",
            compiled_rules.join("|")
        );

        TccSegmenter {
            tcc_regex: Regex::new(&master_pattern).unwrap(),
        }
    }

    /// Finds unbreakable TCC boundaries as raw UTF-8 byte offsets (the regex's native
    /// match offsets). This is the index space the lattice and the daachorse tries operate
    /// in, so slicing `&text[start..end]` against these is always valid.
    pub fn find_byte_positions(&self, text: &str) -> Vec<usize> {
        let mut positions = vec![0usize];
        for mat in self.tcc_regex.find_iter(text) {
            positions.push(mat.end());
        }
        positions
    }

    /// Finds unbreakable TCC boundaries mapped to Unicode character indices.
    ///
    /// This is what the Ruby FFI consumer (`thapthim_tcc_positions`) expects: the Ruby
    /// `tcc_segment` slices the string with `str[start...end]`, which indexes by character.
    pub fn find_positions(&self, text: &str) -> Vec<i32> {
        // Map every char-boundary byte index to its character position. Regex matches always
        // end on a char boundary, so every byte offset we look up here is populated.
        let mut byte_to_char = vec![0i32; text.len() + 1];
        let mut char_count = 0i32;
        for (byte_idx, _) in text.char_indices() {
            byte_to_char[byte_idx] = char_count;
            char_count += 1;
        }
        byte_to_char[text.len()] = char_count; // EOF index anchor

        self.find_byte_positions(text)
            .iter()
            .map(|&byte_pos| byte_to_char[byte_pos])
            .collect()
    }
}
