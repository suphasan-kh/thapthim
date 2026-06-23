// ext/thapthim/src/lm_format.rs
//
// Language-model serialization formats, shared by build.rs (which writes the interned asset) and
// the runtime (which reads it) so the two can never drift. `#[path]`-included by build.rs.
//
//  - LayerCounts / MasterLanguageModel: the String-keyed form emitted by the corpus pipeline
//    (assets/joint_lm.bin). HashMap hasher is irrelevant to bincode bytes, so plain std maps
//    deserialize the FxHashMap-serialized asset fine.
//  - InternedLayer / InternedModel: the compact shipped form. Tokens become dense u32 ids and each
//    bigram key is packed into a u64 (w1_id << 32 | w2_id), replacing ~884k heap String keys.
//
// `intern_model` is the lossless conversion between them (build.rs runs it).
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct LayerCounts {
    pub unigrams: HashMap<String, (usize, usize)>, // token -> (count, preceding_contexts)
    pub bigrams: HashMap<String, usize>,           // "w1\tw2" -> count
}

#[derive(Serialize, Deserialize)]
pub struct MasterLanguageModel {
    pub words: LayerCounts,
    pub syllables: LayerCounts,
    pub tccs: LayerCounts,
}

#[derive(Serialize, Deserialize)]
pub struct InternedLayer {
    pub vocab: Vec<String>,        // id -> token (index is the id)
    pub unigrams: Vec<(u32, u32)>, // id -> (count, preceding_contexts); dense, len == vocab.len()
    pub bigrams: Vec<(u64, u32)>,  // (w1_id << 32 | w2_id, count); Vec on disk, rebuilt to a map at load
}

#[derive(Serialize, Deserialize)]
pub struct InternedModel {
    pub words: InternedLayer,
    pub syllables: InternedLayer,
    pub tccs: InternedLayer,
}

fn id(tok: &str, id_of: &mut HashMap<String, u32>, vocab: &mut Vec<String>) -> u32 {
    match id_of.get(tok) {
        Some(&i) => i,
        None => {
            let i = vocab.len() as u32;
            vocab.push(tok.to_string());
            id_of.insert(tok.to_string(), i);
            i
        }
    }
}

/// Lossless re-encode of one layer: assign dense ids (unigram tokens first, then bigram-only
/// tokens), pack bigram keys to u64, and build a dense unigram table indexed by id.
pub fn intern_layer(layer: &LayerCounts) -> InternedLayer {
    let mut id_of: HashMap<String, u32> = HashMap::new();
    let mut vocab: Vec<String> = Vec::new();

    for k in layer.unigrams.keys() {
        id(k, &mut id_of, &mut vocab);
    }
    let mut bigrams: Vec<(u64, u32)> = Vec::with_capacity(layer.bigrams.len());
    for (key, &count) in &layer.bigrams {
        let (w1, w2) = key.split_once('\t').expect("bigram key is w1\\tw2");
        let i1 = id(w1, &mut id_of, &mut vocab) as u64;
        let i2 = id(w2, &mut id_of, &mut vocab) as u64;
        bigrams.push((i1 << 32 | i2, count as u32));
    }

    let mut unigrams = vec![(0u32, 0u32); vocab.len()];
    for (k, &(c, p)) in &layer.unigrams {
        unigrams[id_of[k] as usize] = (c as u32, p as u32);
    }

    InternedLayer { vocab, unigrams, bigrams }
}

pub fn intern_model(lm: &MasterLanguageModel) -> InternedModel {
    InternedModel {
        words: intern_layer(&lm.words),
        syllables: intern_layer(&lm.syllables),
        tccs: intern_layer(&lm.tccs),
    }
}

// --- Build the LM from the corpus pipeline's human-readable Kneser-Ney count files ---------------
//
// The notebook emits, per tier, two TAB-separated files (LST20-train counts):
//   kn_<tier>_unigrams.txt : token \t count \t preceding_contexts   (N₁₊(• token), the KN cont. count)
//   kn_<tier>_bigrams.txt  : w1    \t w2    \t count
// The space token is the literal " " and is significant (sentence-boundary context), so lines are
// split on '\t' WITHOUT trimming. This is the in-repo replacement for the formerly-external step
// that produced joint_lm.bin.

fn parse_unigrams(path: &str) -> HashMap<String, (usize, usize)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut m = HashMap::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let mut it = line.split('\t');
        if let (Some(tok), Some(c), Some(p)) = (it.next(), it.next(), it.next()) {
            if let (Ok(c), Ok(p)) = (c.parse(), p.parse()) {
                m.insert(tok.to_string(), (c, p));
            }
        }
    }
    m
}

fn parse_bigrams(path: &str) -> HashMap<String, usize> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut m = HashMap::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let mut it = line.split('\t');
        if let (Some(w1), Some(w2), Some(c)) = (it.next(), it.next(), it.next()) {
            if let Ok(c) = c.parse() {
                m.insert(format!("{w1}\t{w2}"), c); // key matches score_transition's "w1\tw2"
            }
        }
    }
    m
}

/// Assemble the full `MasterLanguageModel` from the six `kn_<tier>_{unigrams,bigrams}.txt` files in
/// `dir`. Inverse of the bincode export: this is what regenerates joint_lm.bin from the notebook's
/// output.
pub fn build_lm_from_kn(dir: &str) -> MasterLanguageModel {
    let layer = |tier: &str| LayerCounts {
        unigrams: parse_unigrams(&format!("{dir}/kn_{tier}_unigrams.txt")),
        bigrams: parse_bigrams(&format!("{dir}/kn_{tier}_bigrams.txt")),
    };
    MasterLanguageModel { words: layer("words"), syllables: layer("syllables"), tccs: layer("tccs") }
}
