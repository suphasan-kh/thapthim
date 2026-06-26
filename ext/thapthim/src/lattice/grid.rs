// ext/thapthim/src/lattice/grid.rs
//
// Task-agnostic grid Viterbi: the segmentation engine's DP, lifted off its word/syllable
// specifics. An `Edge<P>` is a grid-aligned span carrying a task payload; a `LatticeModel`
// supplies the first-order cost (a per-node "context" plus a transition price, mirroring the
// Kneser-Ney bigram). `viterbi` returns the maximum-score path. Word/syllable segmentation is the
// first instantiation (see decode.rs `BigramModel`); G2P and spelling correction are future ones,
// differing only in their candidate generation and `LatticeModel` impl — never this file.
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
/// (`Option<u32>`). Each node's `Ctx` is resolved once by the candidate builder and passed to
/// `viterbi` as the `ctx` slice, so this trait supplies only the start context and the costs.
pub trait LatticeModel {
    type Payload;
    type Ctx: Copy;

    /// Context active at a region's start — the sentence-initial token (`" "` in the LM today).
    fn start_ctx(&self) -> Self::Ctx;

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
/// `Scratch` serves every `LatticeModel`. The predecessor (`ending_at`) and processing-order
/// (`starting_at`) buckets live here too, both indexed by *dense grid index* (not byte position) so
/// a lookup is an array index, never a hash; the `touched_*` lists record which buckets a call
/// filled so resets touch only those, reusing every bucket `Vec`'s capacity. NB `viterbi` must not
/// recurse on one thread (it does not: each `segment_*` decodes sequentially); a nested call would
/// panic borrowing this.
#[derive(Default)]
struct Scratch {
    order: Vec<usize>,
    dp: Vec<f64>,
    bp: Vec<Option<usize>>,
    reached: Vec<bool>,
    ending_at: Vec<Vec<usize>>,
    starting_at: Vec<Vec<usize>>,
    touched_end: Vec<usize>,
    touched_start: Vec<usize>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// Exact first-order Viterbi over the edges confined to `[rs, re)`, returning the maximum-score
/// path as edge indices into `edges`. A node's predecessors are the edges ending at its start, so
/// ascending-start order scores them first; `reached` stays a separate flag because a legitimate
/// transition may score `NEG_INFINITY` (a seen token with no followers and a zero bigram count).
/// `ctx[i]` is `edges[i]`'s context (built by the caller in `build_lattice`), taken as a parameter
/// so it can be built once and reused across several regions (the OOV-span syllable decode does
/// exactly that).
/// `pos_idx` maps a byte offset to its dense grid index (the caller's `byte_to_idx`); every edge
/// endpoint and the region bounds `rs`/`re` are grid points, so their `pos_idx` entries are valid.
/// `n_positions` is the grid size (`positions.len()`), sizing the `ending_at` bucket array.
pub fn viterbi<M: LatticeModel>(
    edges: &[Edge<M::Payload>],
    ctx: &[M::Ctx],
    rs: usize,
    re: usize,
    pos_idx: &[u32],
    n_positions: usize,
    model: &M,
) -> Vec<usize> {
    if rs >= re {
        return Vec::new();
    }
    let start = model.start_ctx();
    let n = edges.len();

    SCRATCH.with_borrow_mut(|s| {
        let Scratch { order, dp, bp, reached, ending_at, starting_at, touched_end, touched_start } = s;

        // Flat buckets keyed by dense grid index — no hashing. Reset only the predecessor buckets
        // filled last call (`starting_at` is reset during the flatten below); `touched_start` is
        // cleared here and rebuilt during the fill.
        if ending_at.len() < n_positions {
            ending_at.resize_with(n_positions, Vec::new);
        }
        if starting_at.len() < n_positions {
            starting_at.resize_with(n_positions, Vec::new);
        }
        for &b in touched_end.iter() {
            ending_at[b].clear();
        }
        touched_end.clear();
        touched_start.clear();

        // Single ascending-node-index pass: bucket each in-region edge by dense END index
        // (predecessor lists) and by dense START index (processing order). Index-order fill keeps
        // every bucket index-ordered — the predecessor tie-break the DP below relies on.
        for i in 0..n {
            if edges[i].start < rs || edges[i].end > re {
                continue;
            }
            let e = pos_idx[edges[i].end] as usize;
            if ending_at[e].is_empty() {
                touched_end.push(e);
            }
            ending_at[e].push(i);
            let st = pos_idx[edges[i].start] as usize;
            if starting_at[st].is_empty() {
                touched_start.push(st);
            }
            starting_at[st].push(i);
        }

        // Processing order = nodes by ascending dense start index. Counting sort: sort only the
        // distinct start buckets (far fewer than nodes), then concatenate — O(k log k + n) vs the
        // old O(n log n). Equal-start nodes are independent, so their order never affects the path.
        // Each `starting_at` bucket is cleared as it is drained, leaving the array clean next call.
        touched_start.sort_unstable();
        order.clear();
        for &b in touched_start.iter() {
            for &i in &starting_at[b] {
                order.push(i);
            }
            starting_at[b].clear();
        }

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
                for &p in &ending_at[pos_idx[edges[i].start] as usize] {
                    if reached[p] {
                        let s = dp[p] + model.transition(ctx[p], ctx[i]) + local;
                        if b.is_none_or(|(bs, _)| s > bs) {
                            b = Some((s, Some(p)));
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
        for &i in &ending_at[pos_idx[re] as usize] {
            if reached[i] && best_end.is_none_or(|(bs, _)| dp[i] > bs) {
                best_end = Some((dp[i], i));
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
