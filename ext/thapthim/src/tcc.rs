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

        // Two protective rules, placed first so they win over the `.` single-char fallback:
        //  1. `<[^<>]*>` keeps an angle-bracket tag (e.g. <NE>, </NE>, <Hello>) as ONE cluster.
        //  2. `[A-Za-z0-9_.,@:/#-]+` keeps a contiguous Latin/digit run — including punctuation
        //     wedged between alphanumerics (3.5, URLs, @handles, #tags, ranges) — as ONE cluster,
        //     matching ssg's western-token convention. Both the word and syllable decodes build on
        //     this shared grid, so it groups consistently for both (modern text gain; see below).
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
            r"<[^<>]*>|[A-Za-z0-9_.,@:/#-]+|[๐-๙]+(?:[.,:][๐-๙]+)*|{}|[\s\S]",
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
