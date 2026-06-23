// Builds the embedded LM asset from the corpus pipeline's output, entirely in-repo:
//
//   kn_<tier>_{unigrams,bigrams}.txt  --build_lm_from_kn-->  joint_lm.bin  --intern_model-->  joint_lm_interned.bin
//                  (notebook output)                       (String-keyed)                    (embedded via include_bytes!)
//
// Each step regenerates only when its inputs are newer (or its output is missing), so normal
// incremental builds do no work. A checkout that ships only a downstream artifact (e.g. just the
// committed interned .bin) still builds: missing upstream inputs simply skip their step.
#[path = "src/lm_format.rs"]
mod lm_format;

use lm_format::{InternedModel, MasterLanguageModel};
use std::path::Path;
use std::time::SystemTime;

const KN_FILES: [&str; 6] = [
    "kn_words_unigrams", "kn_words_bigrams",
    "kn_syllables_unigrams", "kn_syllables_bigrams",
    "kn_tccs_unigrams", "kn_tccs_bigrams",
];

fn mtime(p: &str) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

// True when `out` must be rebuilt: it's missing, or `newest_input` is newer than it.
fn stale(out: &str, newest_input: Option<SystemTime>) -> bool {
    !Path::new(out).exists() || matches!((newest_input, mtime(out)), (Some(i), Some(o)) if i > o)
}

fn main() {
    println!("cargo:rerun-if-changed=src");
    for f in KN_FILES {
        println!("cargo:rerun-if-changed=assets/{f}.txt");
    }
    println!("cargo:rerun-if-changed=assets/joint_lm.bin");

    let lm_bin = "assets/joint_lm.bin";
    let interned = "assets/joint_lm_interned.bin";

    // Step 1: kn_*.txt -> joint_lm.bin (String-keyed counts).
    let kn_paths: Vec<String> = KN_FILES.iter().map(|f| format!("assets/{f}.txt")).collect();
    if kn_paths.iter().all(|p| Path::new(p).exists()) {
        let newest_kn = kn_paths.iter().filter_map(|p| mtime(p)).max();
        if stale(lm_bin, newest_kn) {
            let lm = lm_format::build_lm_from_kn("assets");
            std::fs::write(lm_bin, bincode::serialize(&lm).expect("serialize joint_lm.bin"))
                .expect("write joint_lm.bin");
        }
    }

    // Step 2: joint_lm.bin -> joint_lm_interned.bin (compact embedded form).
    if Path::new(lm_bin).exists() && stale(interned, mtime(lm_bin)) {
        let bytes = std::fs::read(lm_bin).expect("read joint_lm.bin");
        let lm: MasterLanguageModel = bincode::deserialize(&bytes).expect("deserialize joint_lm.bin");
        let model: InternedModel = lm_format::intern_model(&lm);
        std::fs::write(interned, bincode::serialize(&model).expect("serialize interned"))
            .expect("write joint_lm_interned.bin");
    }
}
