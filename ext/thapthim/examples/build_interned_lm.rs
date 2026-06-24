// Mint an interned LM asset from a directory of Kneser-Ney count files.
//
//   cargo run --example build_interned_lm -- <kn_dir> <out.bin>
//
// <kn_dir> must contain the six kn_<tier>_{unigrams,bigrams}.txt files (same format build.rs
// consumes from assets/). This is the same two-step build.rs runs — build_lm_from_kn then
// intern_model — exposed as a one-off so we can produce an alternate-corpus LM (e.g. a
// BEST-trained joint_lm_interned_best.bin) without disturbing the default assets/ pipeline.
use thapthim::lm_format::{build_lm_from_kn, intern_model};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: build_interned_lm <kn_dir> <out.bin>");
    let out = args.next().expect("usage: build_interned_lm <kn_dir> <out.bin>");

    let lm = build_lm_from_kn(&dir);
    let model = intern_model(&lm);
    let bytes = bincode::serialize(&model).expect("serialize interned model");
    std::fs::write(&out, &bytes).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {out} ({} bytes)", bytes.len());
}
