// Mint a POS HMM model asset from the LST20 tagged corpus.
//
//   cargo run --release --example train_pos_hmm -- <lst20_train_dir> <out.bin>
//
// <lst20_train_dir> is a directory of LST20 .txt files (e.g. datasets/LST20_full_train). Each line
// is TAB-separated `token \t POS \t NER \t clause`; a blank line ends a sentence. We read columns 1
// (token) and 2 (POS) only. Mirrors examples/build_interned_lm.rs: a one-off offline builder that
// produces an asset the runtime later embeds/loads.
//
// The model: a first-order (bigram) HMM. Transitions and the sentence-initial distribution are
// estimated by add-λ smoothing; emissions by Witten-Bell, which also yields a tag-discriminative
// unknown-word distribution. Surfaces are canonicalized (convention 1) so the space token is one
// consistent vocabulary entry at train and inference.
use std::collections::HashMap;
use std::path::Path;
use thapthim::pos::{canon_surface, Pos, PosModelFile, NUM_TAGS};

/// Add-λ smoothing constant for transitions and the initial distribution. The 16×16 transition grid
/// is dense over 2.7M tokens, so this barely moves known-context decisions; it only keeps unseen
/// (prev, cur) pairs from becoming log(0) = -inf (an outright ban). Small on purpose.
const LAMBDA: f64 = 0.1;

/// Raw counts accumulated over the corpus — the sufficient statistics for a bigram HMM.
#[derive(Default)]
struct Counts {
    /// init[tag]: times `tag` began a sentence (the BOS→tag transition).
    init: [u64; NUM_TAGS],
    /// trans[prev*T + cur]: adjacent (prev, cur) tag pairs within a sentence.
    trans: Vec<u64>, // sized T*T at construction
    /// emit[tag]: canonical word → times seen tagged `tag`.
    emit: Vec<HashMap<String, u64>>, // len T
    /// tag[tag]: total occurrences of `tag` (the emission denominator's token term).
    tag: [u64; NUM_TAGS],
}

impl Counts {
    fn new() -> Self {
        Counts {
            trans: vec![0; NUM_TAGS * NUM_TAGS],
            emit: (0..NUM_TAGS).map(|_| HashMap::new()).collect(),
            ..Default::default()
        }
    }

    /// Fold one sentence (a list of (canonical word, tag) pairs) into the counts.
    fn add_sentence(&mut self, sent: &[(String, Pos)]) {
        let mut prev: Option<usize> = None;
        for (word, pos) in sent {
            let t = pos.idx();
            self.tag[t] += 1;
            *self.emit[t].entry(word.clone()).or_insert(0) += 1;
            match prev {
                None => self.init[t] += 1,
                Some(p) => self.trans[p * NUM_TAGS + t] += 1,
            }
            prev = Some(t);
        }
    }
}

/// Parse every `.txt` under `dir` into counts. Blank line = sentence boundary; surfaces are
/// canonicalized (convention 1: `_`/whitespace → single space). Malformed / unknown-POS lines are
/// skipped so one bad row can't poison the model.
fn collect_counts(dir: &Path) -> Counts {
    let mut counts = Counts::new();
    let mut skipped = 0u64;
    let mut files = 0u64;

    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }
        files += 1;
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));

        let mut sent: Vec<(String, Pos)> = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                if !sent.is_empty() {
                    counts.add_sentence(&sent);
                    sent.clear();
                }
                continue;
            }
            let mut cols = line.split('\t');
            let (Some(token), Some(pos_str)) = (cols.next(), cols.next()) else {
                skipped += 1;
                continue;
            };
            match Pos::from_str(pos_str) {
                Some(pos) => sent.push((canon_surface(token).to_string(), pos)),
                None => skipped += 1,
            }
        }
        if !sent.is_empty() {
            counts.add_sentence(&sent); // final sentence if the file lacks a trailing blank line
        }
    }

    let total: u64 = counts.tag.iter().sum();
    eprintln!("read {files} files, {total} tokens, skipped {skipped} malformed lines");
    counts
}

/// Turn raw counts into the serialized, log-space `PosModelFile`.
///
/// Transitions / init: add-λ MLE. Emission: Witten-Bell — for a tag with `C` tokens over `V`
/// distinct types, a seen word gets `count / (C + V)` and the reserved unseen mass is `V / (C + V)`.
/// That reserved mass IS the per-tag UNK emission, and because open classes accrue far more types
/// than closed ones it is naturally tag-discriminative (an unknown word looks far more like an NN or
/// VV than an NG or CL) — the signal a uniform OOV fallback discards.
fn build_model(counts: &Counts) -> PosModelFile {
    let t = NUM_TAGS;

    // --- initial distribution: P(tag | BOS), add-λ ---
    let n_sent: u64 = counts.init.iter().sum();
    let init_denom = n_sent as f64 + LAMBDA * t as f64;
    let log_init: Vec<f32> = (0..t)
        .map(|s| (((counts.init[s] as f64 + LAMBDA) / init_denom).ln()) as f32)
        .collect();

    // --- transitions: P(cur | prev), add-λ, row-major prev*T + cur ---
    let mut log_trans = vec![0f32; t * t];
    for p in 0..t {
        let row_total: u64 = (0..t).map(|c| counts.trans[p * t + c]).sum();
        let denom = row_total as f64 + LAMBDA * t as f64;
        for c in 0..t {
            let pr = (counts.trans[p * t + c] as f64 + LAMBDA) / denom;
            log_trans[p * t + c] = pr.ln() as f32;
        }
    }

    // --- vocab: a dense id per canonical surface seen under any tag ---
    let mut vocab: HashMap<String, u32> = HashMap::new();
    for tag_tbl in &counts.emit {
        for w in tag_tbl.keys() {
            let id = vocab.len() as u32;
            vocab.entry(w.clone()).or_insert(id);
        }
    }

    // --- emission: Witten-Bell per tag, plus the reserved UNK mass ---
    let mut log_emit: Vec<HashMap<u32, f32>> = vec![HashMap::new(); t];
    let mut log_unk_emit = vec![0f32; t];
    for tag in 0..t {
        let c_tag = counts.tag[tag] as f64; // tokens with this tag
        let v_tag = counts.emit[tag].len() as f64; // distinct types with this tag
        let denom = c_tag + v_tag;
        if denom == 0.0 {
            // Tag never observed at all — keep it barely possible so a smoothed transition can still
            // reach it, but effectively zero-emission.
            log_unk_emit[tag] = (1e-12f64).ln() as f32;
            continue;
        }
        for (w, &cnt) in &counts.emit[tag] {
            let wid = vocab[w];
            log_emit[tag].insert(wid, ((cnt as f64) / denom).ln() as f32);
        }
        log_unk_emit[tag] = (v_tag / denom).ln() as f32;
    }

    eprintln!(
        "model: {} vocab types, {} sentences; UNK emission NN={:.2} VV={:.2} NG={:.2} CL={:.2}",
        vocab.len(),
        n_sent,
        log_unk_emit[Pos::NN.idx()],
        log_unk_emit[Pos::VV.idx()],
        log_unk_emit[Pos::NG.idx()],
        log_unk_emit[Pos::CL.idx()],
    );

    PosModelFile { log_init, log_trans, vocab, log_emit, log_unk_emit }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: train_pos_hmm <lst20_train_dir> <out.bin>");
    let out = args.next().expect("usage: train_pos_hmm <lst20_train_dir> <out.bin>");

    let counts = collect_counts(Path::new(&dir));
    let model = build_model(&counts);
    let bytes = bincode::serialize(&model).expect("serialize pos model");
    std::fs::write(&out, &bytes).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {out} ({} bytes)", bytes.len());
}
