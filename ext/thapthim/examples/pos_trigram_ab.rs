// A/B experiment: does a second-order (trigram) HMM beat the shipped bigram on LST20 POS?
//
//   cargo run --release --example pos_trigram_ab -- datasets/LST20_full_train datasets/LST20_full_test
//
// Emission is held FIXED — loaded from the shipped model (assets/pos_hmm.bin) and reused for both
// arms via `HmmTagger::emission_logprob` — so the only variable is the tag-transition order. The
// bigram arm is the shipped decode (`tag`); the trigram arm decodes with P(t | t-2, t-1) smoothed by
// Brants (2000) deleted interpolation (the standard TnT smoothing, so trigram gets a fair shot).
// Scoring excludes the space token (PU) and splits known vs OOV, matching test/eval_pos.rb.
use std::collections::HashSet;
use thapthim::pos::{canon_surface, HmmTagger, Pos, NUM_TAGS};

const T: usize = NUM_TAGS;

/// Parse an LST20 .txt dir into (canonical words, gold tag ids) per sentence; blank line = boundary.
fn read_sentences(dir: &str) -> Vec<(Vec<String>, Vec<usize>)> {
    let mut out = Vec::new();
    let mut ws: Vec<String> = Vec::new();
    let mut ts: Vec<usize> = Vec::new();
    for path in std::fs::read_dir(dir).expect("read_dir").flatten() {
        let p = path.path();
        if p.extension().and_then(|s| s.to_str()) != Some("txt") {
            continue;
        }
        for line in std::fs::read_to_string(&p).expect("read").lines() {
            if line.trim().is_empty() {
                if !ws.is_empty() {
                    out.push((std::mem::take(&mut ws), std::mem::take(&mut ts)));
                }
                continue;
            }
            let mut c = line.split('\t');
            if let (Some(w), Some(pos)) = (c.next(), c.next())
                && let Some(t) = Pos::from_str(pos)
            {
                ws.push(canon_surface(w).to_string());
                ts.push(t.idx());
            }
        }
        if !ws.is_empty() {
            out.push((std::mem::take(&mut ws), std::mem::take(&mut ts)));
        }
    }
    out
}

#[derive(Default)]
struct Acc {
    n: u64,
    c: u64,
    kn: u64,
    kc: u64,
    on: u64,
    oc: u64,
}
impl Acc {
    fn add(&mut self, ok: bool, known: bool) {
        self.n += 1;
        self.c += ok as u64;
        if known {
            self.kn += 1;
            self.kc += ok as u64;
        } else {
            self.on += 1;
            self.oc += ok as u64;
        }
    }
    fn report(&self, name: &str) {
        let p = |a: u64, b: u64| if b == 0 { 0.0 } else { 100.0 * a as f64 / b as f64 };
        println!(
            "{name:8}  overall {:.2}%   known {:.2}%   OOV {:.2}%",
            p(self.c, self.n),
            p(self.kc, self.kn),
            p(self.oc, self.on)
        );
    }
}

fn main() {
    let mut a = std::env::args().skip(1);
    let train_dir = a.next().unwrap_or_else(|| "datasets/LST20_full_train".into());
    let test_dir = a.next().unwrap_or_else(|| "datasets/LST20_full_test".into());

    let train = read_sentences(&train_dir);
    let test = read_sentences(&test_dir);

    // --- counts over training tag sequences ---
    let mut uni = vec![0u64; T];
    let mut bi = vec![0u64; T * T];
    let mut tri = vec![0u64; T * T * T];
    let mut init1 = vec![0u64; T];
    let mut ntok = 0u64;
    let mut known: HashSet<String> = HashSet::new();
    for (ws, ts) in &train {
        for w in ws {
            known.insert(w.clone());
        }
        for (i, &t) in ts.iter().enumerate() {
            uni[t] += 1;
            ntok += 1;
            if i == 0 {
                init1[t] += 1;
            }
            if i >= 1 {
                bi[ts[i - 1] * T + t] += 1;
            }
            if i >= 2 {
                tri[(ts[i - 2] * T + ts[i - 1]) * T + t] += 1;
            }
        }
    }

    // --- Brants (2000) deleted interpolation for the trigram weights ---
    let (mut l1, mut l2, mut l3) = (0.0f64, 0.0f64, 0.0f64);
    for w in 0..T {
        for u in 0..T {
            for v in 0..T {
                let c3 = tri[(w * T + u) * T + v];
                if c3 == 0 {
                    continue;
                }
                let f3 = if bi[w * T + u] > 1 {
                    (c3 as f64 - 1.0) / (bi[w * T + u] as f64 - 1.0)
                } else {
                    0.0
                };
                let f2 = if uni[u] > 1 {
                    (bi[u * T + v] as f64 - 1.0) / (uni[u] as f64 - 1.0)
                } else {
                    0.0
                };
                let f1 = if ntok > 1 { (uni[v] as f64 - 1.0) / (ntok as f64 - 1.0) } else { 0.0 };
                if f3 >= f2 && f3 >= f1 {
                    l3 += c3 as f64;
                } else if f2 >= f1 {
                    l2 += c3 as f64;
                } else {
                    l1 += c3 as f64;
                }
            }
        }
    }
    let s = l1 + l2 + l3;
    l1 /= s;
    l2 /= s;
    l3 /= s;
    println!("deleted-interpolation weights: λ1(uni)={l1:.3}  λ2(bi)={l2:.3}  λ3(tri)={l3:.3}");

    // --- precompute log-prob tables ---
    let lam = 0.1; // add-λ for init and the position-1 bigram (matches the shipped trainer)
    let init_log: Vec<f64> = (0..T)
        .map(|t| ((init1[t] as f64 + lam) / (train.len() as f64 + lam * T as f64)).ln())
        .collect();
    let bi_log: Vec<f64> = (0..T * T)
        .map(|k| ((bi[k] as f64 + lam) / (uni[k / T] as f64 + lam * T as f64)).ln())
        .collect();
    let p_uni = |v: usize| uni[v] as f64 / ntok as f64;
    let p_bi = |u: usize, v: usize| if uni[u] > 0 { bi[u * T + v] as f64 / uni[u] as f64 } else { 0.0 };
    let mut tri_log = vec![f64::NEG_INFINITY; T * T * T];
    for w in 0..T {
        for u in 0..T {
            for v in 0..T {
                let pt = if bi[w * T + u] > 0 {
                    tri[(w * T + u) * T + v] as f64 / bi[w * T + u] as f64
                } else {
                    0.0
                };
                let p = l3 * pt + l2 * p_bi(u, v) + l1 * p_uni(v);
                tri_log[(w * T + u) * T + v] = if p > 0.0 { p.ln() } else { f64::NEG_INFINITY };
            }
        }
    }

    let tagger = HmmTagger::from_bytes(include_bytes!("../assets/pos_hmm.bin"));

    // --- decode + score both arms, timing each decode separately ---
    let (mut big, mut trg) = (Acc::default(), Acc::default());
    let (mut tb, mut tt) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for (ws, gold) in &test {
        let s = std::time::Instant::now();
        let pb = tagger.tag(ws);
        tb += s.elapsed();
        let s = std::time::Instant::now();
        let pt = trigram_decode(ws, &tagger, &init_log, &bi_log, &tri_log);
        tt += s.elapsed();
        for i in 0..ws.len() {
            if ws[i] == " " {
                continue; // exclude the space token (PU), as in eval_pos.rb
            }
            let kn = known.contains(&ws[i]);
            big.add(pb[i].idx() == gold[i], kn);
            trg.add(pt[i] == gold[i], kn);
        }
    }
    let all: usize = test.iter().map(|(w, _)| w.len()).sum();
    println!("(scored tokens: {}, spaces excluded)", big.n);
    big.report("bigram");
    trg.report("trigram");
    println!(
        "decode speed (all {all} tokens):  bigram {:.0} tok/s   trigram {:.0} tok/s   ({:.1}× slower)",
        all as f64 / tb.as_secs_f64(),
        all as f64 / tt.as_secs_f64(),
        tt.as_secs_f64() / tb.as_secs_f64()
    );
}

/// Second-order Viterbi over states = (prev tag, cur tag). Emission is the shipped model's; position
/// 0 uses the initial distribution, position 1 a smoothed bigram, positions ≥2 the interpolated
/// trigram. Returns one tag id per token.
fn trigram_decode(
    words: &[String],
    tagger: &HmmTagger,
    init_log: &[f64],
    bi_log: &[f64],
    tri_log: &[f64],
) -> Vec<usize> {
    let n = words.len();
    if n == 0 {
        return Vec::new();
    }
    let t = T;
    let mut em = vec![0f64; n * t];
    for i in 0..n {
        for v in 0..t {
            em[i * t + v] = tagger.emission_logprob(&words[i], v);
        }
    }

    // position 0
    let mut best0 = vec![f64::NEG_INFINITY; t];
    for v in 0..t {
        best0[v] = init_log[v] + em[v];
    }
    if n == 1 {
        let v = (0..t).max_by(|&a, &b| best0[a].total_cmp(&best0[b])).unwrap();
        return vec![v];
    }

    // position 1: dp[(u,v)] = best0[u] + bi(u→v) + em1[v]
    let mut dp = vec![f64::NEG_INFINITY; t * t];
    for u in 0..t {
        if best0[u] == f64::NEG_INFINITY {
            continue;
        }
        for v in 0..t {
            dp[u * t + v] = best0[u] + bi_log[u * t + v] + em[t + v];
        }
    }

    // positions ≥2: bp[i][(u,v)] = the tag at i-2 on the best path ending (u at i-1, v at i)
    let mut bp: Vec<Vec<u8>> = vec![Vec::new(); n];
    for i in 2..n {
        let mut ndp = vec![f64::NEG_INFINITY; t * t];
        let mut nbp = vec![0u8; t * t];
        for u in 0..t {
            for v in 0..t {
                let mut best = f64::NEG_INFINITY;
                let mut arg = 0usize;
                for w in 0..t {
                    let d = dp[w * t + u];
                    if d == f64::NEG_INFINITY {
                        continue;
                    }
                    let sc = d + tri_log[(w * t + u) * t + v];
                    if sc > best {
                        best = sc;
                        arg = w;
                    }
                }
                if best != f64::NEG_INFINITY {
                    ndp[u * t + v] = best + em[i * t + v];
                    nbp[u * t + v] = arg as u8;
                }
            }
        }
        dp = ndp;
        bp[i] = nbp;
    }

    // terminate on the best (u,v) at the last position, then walk the w-pointers back
    let (mut bu, mut bv, mut best) = (0usize, 0usize, f64::NEG_INFINITY);
    for u in 0..t {
        for v in 0..t {
            if dp[u * t + v] > best {
                best = dp[u * t + v];
                bu = u;
                bv = v;
            }
        }
    }
    let mut out = vec![0usize; n];
    out[n - 1] = bv;
    out[n - 2] = bu;
    let (mut u, mut v) = (bu, bv);
    for i in (2..n).rev() {
        let w = bp[i][u * t + v] as usize;
        out[i - 2] = w;
        v = u;
        u = w;
    }
    out
}
