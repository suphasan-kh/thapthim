# test/benchmark_accuracy.py
#
# Accuracy comparison of Thai word tokenizers using the RESEARCH-STANDARD metric — the same
# one the AttaCut and DeepCut papers report — implemented by pythainlp.benchmarks. Each sentence
# is scored at two levels (whitespace is stripped first, per the reference preprocessing):
#   - char-level F1  : word-boundary detection (each character labelled start-of-word or not)
#   - word-level F1  : a predicted word is correct iff BOTH its boundaries match the gold word
#
# Every engine goes through the identical scorer. Baselines (newmm, nlpo3, attacut, deepcut)
# are tokenized natively here; Thapthim's predictions are read from a dump produced by
# `ruby test/dump_segmentation.rb <dir>` (so the Ruby/Rust segmenter and the Python baselines
# are judged by one metric). See docs/BENCHMARKS.md for the published results.
#
# Setup (baselines are NOT gem deps; install into a throwaway venv):
#   python3 -m venv /tmp/thai_bench
#   /tmp/thai_bench/bin/pip install "pythainlp[benchmarks]" attacut deepcut nlpo3 tensorflow
#
# Generate Thapthim predictions first:
#   ruby test/dump_segmentation.rb /tmp/pred_lst20
#   THAPTHIM_LM=best ruby test/dump_segmentation.rb /tmp/pred_best   # optional gated BEST LM
#
# Run the scorer:
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py thapthim-LST20 --pred /tmp/pred_lst20
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py attacut
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py deepcut ws1000        # one corpus
#
# NB results are comparable across tools only at the SAME caps and SAME metric. Each model has
# a "home" corpus (its training data) where its score is inflated: Thapthim-LST20 -> LST20,
# Thapthim-BEST/attacut/deepcut -> BEST, nlpo3/newmm -> a LEXiTRON-style dictionary. The fairest
# read is each on its home turf plus the out-of-domain corpora (tnhc, vistec, ws1000).
import json, os, sys, time, itertools
from pythainlp.benchmarks.word_tokenization import compute_stats, preprocessing

DATASETS = os.path.join(os.path.dirname(__file__), "..", "datasets")

# short => (filename, cap). Caps bound deepcut's (very slow) runtime; lst20/tnhc/ws1000 are full
# test sets, best/vistec are capped. Override a single cap with LIMIT=N for quick smoke runs.
CORPORA = [
    ("lst20",  "LST20_test_cleaned.jsonl", 5250),
    ("best",   "BEST_test_cleaned.jsonl",  3000),
    ("vistec", "VISTEC_test.jsonl",        3000),
    ("tnhc",   "tnhc_test.jsonl",          4403),
    ("ws1000", "ws1000.jsonl",             993),
]

def load_gold(fname, n):
    with open(os.path.join(DATASETS, fname)) as f:
        return [json.loads(l) for l in itertools.islice(f, n)]

def f1(p, r):
    return 2 * p * r / (p + r) if (p + r) else 0.0

def score(gold_sents, pred_sents):
    """Aggregate char-level and word-level F1 via the pythainlp reference metric."""
    c_tp = c_fp = c_fn = 0
    w_correct = w_pred = w_ref = 0
    for g, s in zip(gold_sents, pred_sents):
        r, p = preprocessing("|".join(g)), preprocessing("|".join(s))
        if not (r and p):
            continue
        st = compute_stats(r, p)
        c = st["char_level"]
        c_tp += c["tp"]; c_fp += c["fp"]; c_fn += c["fn"]
        w = st["word_level"]
        w_correct += w["correctly_tokenized_words"]
        w_pred += w["total_words_in_sample"]
        w_ref += w["total_words_in_ref_sample"]
    cf = f1(c_tp / (c_tp + c_fp) if c_tp + c_fp else 0, c_tp / (c_tp + c_fn) if c_tp + c_fn else 0)
    wf = f1(w_correct / w_pred if w_pred else 0, w_correct / w_ref if w_ref else 0)
    return cf, wf

def native_tokenizer(engine):
    if engine in ("nlpo3", "newmm"):
        from pythainlp.tokenize import word_tokenize
        return lambda s: word_tokenize(s, engine=engine)
    if engine == "attacut":
        from attacut import Tokenizer
        return Tokenizer(model="attacut-sc").tokenize
    if engine == "deepcut":
        import deepcut
        try:  # silence Keras's per-predict progress bars (one per sentence -> MBs of noise)
            import tensorflow as tf
            tf.keras.utils.disable_interactive_logging()
        except Exception:
            pass
        return deepcut.tokenize
    raise SystemExit(f"unknown engine: {engine}")

def main():
    argv = sys.argv[1:]
    pred_dir = None
    if "--pred" in argv:
        i = argv.index("--pred")
        pred_dir = argv[i + 1]
        del argv[i:i + 2]  # drop the flag AND its value before reading positionals
    args = [a for a in argv if not a.startswith("--")]
    if not args:
        raise SystemExit("usage: benchmark_accuracy.py <engine> [corpus] [--pred <dir>]")
    engine = args[0]
    only = args[1] if len(args) > 1 else None
    limit_env = int(os.environ["LIMIT"]) if "LIMIT" in os.environ else None

    is_thapthim = engine.lower().startswith("thapthim")
    if is_thapthim and not pred_dir:
        raise SystemExit("thapthim engines need --pred <dir> (run test/dump_segmentation.rb first)")
    tok = None if is_thapthim else native_tokenizer(engine)

    for short, fname, cap in CORPORA:
        if only and short != only:
            continue
        lim = limit_env or cap
        gold = load_gold(fname, lim)
        texts = ["".join(g) for g in gold]
        if is_thapthim:
            with open(os.path.join(pred_dir, f"{short}.jsonl")) as f:
                pred = [json.loads(l) for l in itertools.islice(f, lim)]
            cps = None
        else:
            for t in texts[:100]:
                tok(t)  # warmup
            t0 = time.perf_counter()
            pred = [tok(t) for t in texts]
            cps = sum(len(t) for t in texts) / (time.perf_counter() - t0)
        cf1, wf1 = score(gold, pred)
        recon = sum(1 for t, p in zip(texts, pred) if "".join(p) != t)
        spd = f"{cps:9.0f} char/s" if cps else "   (see Ruby)"
        print(f"{engine:14s} {short:7s} n={len(gold):5d} | char-F1={cf1:.4f}  word-F1={wf1:.4f} "
              f"| recon_mism={recon:4d} | {spd}", flush=True)

if __name__ == "__main__":
    main()
