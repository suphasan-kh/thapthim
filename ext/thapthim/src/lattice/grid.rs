// ext/thapthim/src/lattice/grid.rs
//
// Task-agnostic grid Viterbi: the segmentation engine's DP, lifted off its word/syllable
// specifics. An `Edge<P>` is a grid-aligned span carrying a task payload; a `LatticeModel`
// supplies the first-order cost (a per-node "context" plus a transition price, mirroring the
// Kneser-Ney bigram). `viterbi` returns the maximum-score path. Word/syllable segmentation is the
// first instantiation (see decode.rs `BigramModel`); G2P and spelling correction are future ones,
// differing only in their candidate generation and `LatticeModel` impl — never this file.
use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// A grid-aligned candidate: occupies byte span `[start, end)` on the TCC grid, carrying a
/// task-specific payload (the lattice tier for segmentation; a reading/correction elsewhere).
#[derive(Clone)]
pub struct Edge<P> {
    pub start: usize,
    pub end: usize,
    pub payload: P,
}

/// A first-order (bigram) cost over a grid lattice. `Ctx` is the per-node state a node exposes as
/// the "previous" token for the next transition — for the bigram LM that is the token id
/// (`Option<u32>`), resolved once per node by `contexts`, exactly as the old `candidate_ids` did.
pub trait LatticeModel {
    type Payload;
    type Ctx: Copy;

    /// Context active at a region's start — the sentence-initial token (`" "` in the LM today).
    fn start_ctx(&self) -> Self::Ctx;

    /// Resolve every edge's context once, up front, for the decode hot path (no per-edge hashing).
    fn contexts(&self, edges: &[Edge<Self::Payload>]) -> Vec<Self::Ctx>;

    /// Local (emission) score added once when a node is finalised, independent of the predecessor.
    /// `0.0` for plain segmentation; an edit/rule penalty for correction/G2P. Defaulted so the
    /// segmentation model need not implement it (and `+ 0.0` leaves its scores bit-identical).
    fn node_cost(&self, _edge: &Edge<Self::Payload>) -> f64 {
        0.0
    }

    /// First-order transition score from `prev` to `cur` (higher = better; log-prob scale).
    fn transition(&self, prev: Self::Ctx, cur: Self::Ctx) -> f64;
}

/// Reusable per-thread scratch for `viterbi`, so the repeated decodes in one `segment_*` call (the
/// word pass, then one per OOV span) don't re-allocate the DP arrays each time. Every buffer is
/// cleared and refilled per use, so only its capacity — the high-water mark of the largest decode
/// on this thread, a few KB — persists. None of the buffers depend on `M::Ctx`, so one concrete
/// `Scratch` serves every `LatticeModel`. The per-end `ending_at` buckets live here too: clearing
/// the map keeps its table and reuses each bucket `Vec`'s capacity rather than freeing it. NB
/// `viterbi` must not recurse on one thread (it does not: each `segment_*` decodes sequentially);
/// a nested call would panic borrowing this.
#[derive(Default)]
struct Scratch {
    order: Vec<usize>,
    dp: Vec<f64>,
    bp: Vec<Option<usize>>,
    reached: Vec<bool>,
    ending_at: FxHashMap<usize, Vec<usize>>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// Exact first-order Viterbi over the edges confined to `[rs, re)`, returning the maximum-score
/// path as edge indices into `edges`. A node's predecessors are the edges ending at its start, so
/// ascending-start order scores them first; `reached` stays a separate flag because a legitimate
/// transition may score `NEG_INFINITY` (a seen token with no followers and a zero bigram count).
/// `ctx` must be `model.contexts(edges)`, taken as a parameter so a caller can build it once and
/// reuse it across several regions (the OOV-span syllable decode does exactly that).
pub fn viterbi<M: LatticeModel>(
    edges: &[Edge<M::Payload>],
    ctx: &[M::Ctx],
    rs: usize,
    re: usize,
    model: &M,
) -> Vec<usize> {
    if rs >= re {
        return Vec::new();
    }
    let start = model.start_ctx();
    let n = edges.len();

    SCRATCH.with_borrow_mut(|s| {
        let Scratch { order, dp, bp, reached, ending_at } = s;

        // In-region node indices, in ascending index order, used to populate `ending_at` (so each
        // bucket is index-ordered, fixing the predecessor tie-break) before `order` is sorted.
        order.clear();
        order.extend((0..n).filter(|&i| edges[i].start >= rs && edges[i].end <= re));

        // Clear keeps the map's table and each bucket's capacity; `or_default` reuses an emptied
        // bucket when its key recurs, so steady-state decoding allocates no bucket storage.
        for v in ending_at.values_mut() {
            v.clear();
        }
        for &i in order.iter() {
            ending_at.entry(edges[i].end).or_default().push(i);
        }

        // Predecessors have strictly smaller start, and equal-start nodes are independent, so an
        // unstable sort by start is a valid processing order and never perturbs the result.
        order.sort_unstable_by_key(|&i| edges[i].start);

        dp.clear();
        dp.resize(n, f64::NEG_INFINITY);
        bp.clear();
        bp.resize(n, None);
        reached.clear();
        reached.resize(n, false);

        for &i in order.iter() {
            let local = model.node_cost(&edges[i]);
            let best: Option<(f64, Option<usize>)> = if edges[i].start == rs {
                Some((model.transition(start, ctx[i]) + local, None))
            } else {
                let mut b: Option<(f64, Option<usize>)> = None;
                if let Some(prevs) = ending_at.get(&edges[i].start) {
                    for &p in prevs {
                        if reached[p] {
                            let s = dp[p] + model.transition(ctx[p], ctx[i]) + local;
                            if b.is_none_or(|(bs, _)| s > bs) {
                                b = Some((s, Some(p)));
                            }
                        }
                    }
                }
                b
            };
            if let Some((s, prev)) = best {
                dp[i] = s;
                bp[i] = prev;
                reached[i] = true;
            }
        }

        let mut best_end: Option<(f64, usize)> = None;
        if let Some(ends) = ending_at.get(&re) {
            for &i in ends {
                if reached[i] && best_end.is_none_or(|(bs, _)| dp[i] > bs) {
                    best_end = Some((dp[i], i));
                }
            }
        }

        let mut path = Vec::new();
        let mut cur = best_end.map(|(_, i)| i);
        while let Some(i) = cur {
            path.push(i);
            cur = bp[i];
        }
        path.reverse();
        path
    })
}
