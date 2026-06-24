# test/eval_oov_compare.py
#
# Cross-model OOV-stratified recall (SIGHAN-style R_iv / R_oov), one shared scorer.
#
# Companion to test/eval_oov.rb: that script answers "what is Thapthim's OOV recall";
# this answers "is it good COMPARED to other tokenizers", judged fairly.
#
# Fairness design:
#   * ONE shared OOV reference for every model: Thapthim's shipped word lexicon
#     (ext/thapthim/assets/master_words_vocab.txt). A gold word is OOV iff it is not
#     in that set. Every engine is therefore scored on the IDENTICAL set of words,
#     so the comparison is apples-to-apples. (This measures "of the words Thapthim
#     does not know, how well does each model recover them" — exactly the question.)
#   * IDENTICAL gold, caps, span logic and whitespace handling across all engines.
#   * A gold word is recalled iff its exact [start, end) char span is in the prediction.
#     Whitespace-only gold tokens are excluded from both buckets.
#
# Same corpora/caps as benchmark_accuracy.py so numbers line up with docs/BENCHMARKS.md.
#
# Setup (baselines are NOT gem deps; throwaway venv):
#   python3 -m venv /tmp/thai_bench
#   /tmp/thai_bench/bin/pip install "pythainlp[benchmarks]" attacut deepcut nlpo3 tensorflow
#
# Thapthim predictions come from the Ruby dump (judged in this same scorer):
#   ruby test/dump_segmentation.rb /tmp/pred_lst20
#
# Run:
#   /tmp/thai_bench/bin/python test/eval_oov_compare.py attacut
#   /tmp/thai_bench/bin/python test/eval_oov_compare.py thapthim-LST20 --pred /tmp/pred_lst20
#   /tmp/thai_bench/bin/python test/eval_oov_compare.py newmm nlpo3 attacut deepcut
import json, os, sys, itertools

HERE = os.path.dirname(__file__)
DATASETS = os.path.join(HERE, "..", "datasets")
VOCAB = os.environ.get(
    "THAPTHIM_VOCAB",
    os.path.join(HERE, "..", "ext", "thapthim", "assets", "master_words_vocab.txt"),
)

CORPORA = [
    ("lst20",  "LST20_test_cleaned.jsonl", 5250),
    ("best",   "BEST_test_cleaned.jsonl",  3000),
    ("vistec", "VISTEC_test.jsonl",        3000),
    ("tnhc",   "tnhc_test.jsonl",          4403),
    ("ws1000", "ws1000.jsonl",             993),
]

def load_vocab():
    with open(VOCAB, encoding="utf-8") as f:
        return {w.rstrip("\n") for w in f if w.rstrip("\n")}

def load_gold(fname, n):
    with open(os.path.join(DATASETS, fname), encoding="utf-8") as f:
        return [json.loads(l) for l in itertools.islice(f, n)]

def word_spans(tokens):
    """[(word, start, end), ...] for non-whitespace tokens; offsets advance through spaces."""
    out, pos = [], 0
    for tok in tokens:
        n = len(tok)
        if tok.strip():
            out.append((tok, pos, pos + n))
        pos += n
    return out

def pred_span_set(tokens):
    out, pos = set(), 0
    for tok in tokens:
        n = len(tok)
        if tok.strip():
            out.add((pos, pos + n))
        pos += n
    return out

def native_tokenizer(engine):
    if engine in ("nlpo3", "newmm"):
        from pythainlp.tokenize import word_tokenize
        return lambda s: word_tokenize(s, engine=engine)
    if engine == "attacut":
        from attacut import Tokenizer
        return Tokenizer(model="attacut-sc").tokenize
    if engine == "deepcut":
        import deepcut
        try:
            import tensorflow as tf
            tf.keras.utils.disable_interactive_logging()
        except Exception:
            pass
        return deepcut.tokenize
    raise SystemExit(f"unknown engine: {engine}")

def score(engine, gold, pred, vocab):
    iv_t = iv_h = oov_t = oov_h = 0
    for g, p in zip(gold, pred):
        pred_set = pred_span_set(p)
        for word, s, e in word_spans(g):
            hit = (s, e) in pred_set
            if word in vocab:
                iv_t += 1; iv_h += hit
            else:
                oov_t += 1; oov_h += hit
    return iv_t, iv_h, oov_t, oov_h

def main():
    argv = sys.argv[1:]
    pred_dir = None
    if "--pred" in argv:
        i = argv.index("--pred"); pred_dir = argv[i + 1]; del argv[i:i + 2]
    only = None
    if "--corpus" in argv:
        i = argv.index("--corpus"); only = argv[i + 1]; del argv[i:i + 2]
    engines = [a for a in argv if not a.startswith("--")]
    if not engines:
        raise SystemExit("usage: eval_oov_compare.py <engine...> [--pred <dir>] [--corpus <name>]")
    limit_env = int(os.environ["LIMIT"]) if "LIMIT" in os.environ else None

    vocab = load_vocab()
    print(f"OOV reference: {os.path.basename(VOCAB)} ({len(vocab)} words) — shared across all engines")
    print(f"{'engine':16s} {'corpus':7s} {'OOV%':>6s} {'R_oov':>8s} {'R_iv':>8s}   (oov_hit/oov_tot)")
    print("-" * 72)

    for engine in engines:
        is_thap = engine.lower().startswith("thapthim")
        if is_thap and not pred_dir:
            raise SystemExit(f"{engine} needs --pred <dir> (run test/dump_segmentation.rb first)")
        tok = None if is_thap else native_tokenizer(engine)

        agg = [0, 0, 0, 0]
        for short, fname, cap in CORPORA:
            if only and short != only:
                continue
            lim = limit_env or cap
            gold = load_gold(fname, lim)
            if is_thap:
                with open(os.path.join(pred_dir, f"{short}.jsonl"), encoding="utf-8") as f:
                    pred = [json.loads(l) for l in itertools.islice(f, lim)]
            else:
                pred = [tok("".join(g)) for g in gold]

            iv_t, iv_h, oov_t, oov_h = score(engine, gold, pred, vocab)
            for k, v in enumerate((iv_t, iv_h, oov_t, oov_h)):
                agg[k] += v
            r_oov = oov_h / oov_t if oov_t else 0.0
            r_iv = iv_h / iv_t if iv_t else 0.0
            oov_pct = 100 * oov_t / (iv_t + oov_t) if (iv_t + oov_t) else 0.0
            print(f"{engine:16s} {short:7s} {oov_pct:5.1f}% {r_oov:8.4f} {r_iv:8.4f}   ({oov_h}/{oov_t})", flush=True)

        iv_t, iv_h, oov_t, oov_h = agg
        r_oov = oov_h / oov_t if oov_t else 0.0
        r_iv = iv_h / iv_t if iv_t else 0.0
        oov_pct = 100 * oov_t / (iv_t + oov_t) if (iv_t + oov_t) else 0.0
        print(f"{engine:16s} {'ALL':7s} {oov_pct:5.1f}% {r_oov:8.4f} {r_iv:8.4f}   ({oov_h}/{oov_t})  <== micro-avg")
        print("-" * 72)

if __name__ == "__main__":
    main()
