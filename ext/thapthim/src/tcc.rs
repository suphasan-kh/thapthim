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
            "เccีtย(?=[เ-ไก-ฮ]|$)", "เc[ิีุู]tย(?=[เ-ไก-ฮ]|$)", "เcc็ck", "เcิc์ck",
            "เcิtck", "เcีtยะ?", "เcืtอะk", "เcื", "เctา?ะ?", "c[ึื]tck",
            "c[ะ-ู]tk", "c[ิุู]์", "cรรc์", "c็", "ct[ะาำ]?", "แc็ck", "แcc์k",
            "แctะk", "แcc็ck", "แccc์k", "โctะk", "[เ-ไ]ctk", "ก็", "อึ", "หึ",
        ];

        let compiled_rules: Vec<String> = raw_rules
            .iter()
            .map(|rule| rule.replace("c", c).replace("t", t).replace("k", &k))
            .collect();

        // ✅ FIXED: Enforces matching a single fallback character, handling UTF-8 safely
        let master_pattern = format!("{}|.", compiled_rules.join("|"));

        TccSegmenter {
            tcc_regex: Regex::new(&master_pattern).unwrap(),
        }
    }

    pub fn find_positions(&self, text: &str) -> Vec<i32> {
        let mut positions = vec![0];
        for mat in self.tcc_regex.find_iter(text) {
            positions.push(mat.end() as i32);
        }
        positions
    }
}