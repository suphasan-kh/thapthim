# test/benchmark_accuracy.py
#
# Accuracy comparison of external Thai tokenizers against Thapthim, on the SAME gold
# corpora and the SAME word-span F1 metric as test/eval_segment.rb (a predicted word is
# correct iff BOTH of its char-offset boundaries match the gold word). This is the
# accuracy counterpart to benchmark_baselines.py (which measures only speed).
#
# Thapthim's own scores come from `ruby test/eval_segment.rb` — run that separately and
# place its F1 alongside this script's output. The metric is identical, so the numbers are
# directly comparable.
#
# Setup (baselines are NOT a gem dependency; install into a throwaway venv):
#   python3 -m venv /tmp/thai_bench && /tmp/thai_bench/bin/pip install pythainlp attacut
#
# Usage:
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py                  # all engines, all corpora
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py newmm           # one engine
#   /tmp/thai_bench/bin/python test/benchmark_accuracy.py attacut lst20 best
#
# Env:
#   LIMIT=3000   # cap sentences per corpus (match eval_segment.rb's THAPTHIM_EVAL_LIMIT)
#
# NB results are only comparable across tools at the SAME limit and SAME preprocessing.
# Each model also has a "home" corpus (its training data) where its score is inflated:
# Thapthim's LM is trained on LST20; attacut is trained on BEST. The fairest comparison is
# the out-of-domain corpora (tnhc, vistec, ws1000).
import json, os, sys, itertools

DATASETS = os.path.join(os.path.dirname(__file__), "..", "datasets")
LIMIT = int(os.environ.get("LIMIT", "3000"))

# short name => filename (mirrors SegEval::CORPORA in eval_segment.rb, plus ws1000)
CORPORA = {
    "tnhc":   "tnhc_test.jsonl",
    "lst20":  "LST20_test_cleaned.jsonl",
    "best":   "BEST_train_cleaned.jsonl",
    "vistec": "VISTEC_test.jsonl",
    "ws1000": "ws1000.jsonl",
}

def load(name):
    """Read up to LIMIT non-empty token-arrays from a JSONL corpus."""
    gold = []
    with open(os.path.join(DATASETS, CORPORA[name])) as f:
        for line in itertools.islice(f, LIMIT):
            line = line.strip()
            if line:
                arr = json.loads(line)
                if arr:
                    gold.append(arr)
    return gold

def spans(tokens, exclude_space):
    """Port of eval_segment.rb SegEval.spans: the set of [start,end) char-offset word spans.
    With exclude_space, whitespace-only tokens are dropped AFTER offsets are computed."""
    out = set()
    pos = 0
    for tok in tokens:
        L = len(tok)
        if not (exclude_space and tok.strip() == ""):
            out.add((pos, pos + L))
        pos += L
    return out

def f1(tp, fp, fn):
    p = 0.0 if tp + fp == 0 else tp / (tp + fp)
    r = 0.0 if tp + fn == 0 else tp / (tp + fn)
    return 0.0 if p + r == 0 else 2 * p * r / (p + r)

def score(gold, tok):
    tp = fp = fn = tpn = fpn = fnn = mism = 0
    for g in gold:
        text = "".join(g)
        pred = tok(text)
        if "".join(pred) != text:
            mism += 1
        gs, ps = spans(g, False), spans(pred, False)
        hit = len(gs & ps); tp += hit; fp += len(ps) - hit; fn += len(gs) - hit
        gn, pn = spans(g, True), spans(pred, True)
        hitn = len(gn & pn); tpn += hitn; fpn += len(pn) - hitn; fnn += len(gn) - hitn
    return f1(tp, fp, fn), f1(tpn, fpn, fnn), mism

def make_engine(name):
    if name == "newmm":
        from pythainlp.tokenize import word_tokenize
        return lambda s: word_tokenize(s, engine="newmm")
    if name == "attacut":
        from attacut import Tokenizer
        t = Tokenizer(model="attacut-sc")
        t.tokenize("ทดสอบ")  # warmup / load weights
        return t.tokenize
    if name == "deepcut":
        import deepcut
        return deepcut.tokenize
    raise SystemExit(f"unknown engine '{name}' (newmm | attacut | deepcut)")

def main(argv):
    engines = [a for a in argv if a in ("newmm", "attacut", "deepcut")]
    corpora = [a for a in argv if a in CORPORA]
    engines = engines or ["newmm", "attacut"]
    corpora = corpora or list(CORPORA)

    print(f"baseline accuracy vs Thapthim  (LIMIT={LIMIT}, metric: word-span F1)")
    print(f"{'corpus':8s} {'model':10s} {'F1_incl':>8s} {'F1_excl':>8s} {'mismatch':>9s}")
    built = {e: make_engine(e) for e in engines}
    for c in corpora:
        gold = load(c)
        for e in engines:
            fi, fe, mism = score(gold, built[e])
            print(f"{c:8s} {e:10s} {fi:8.4f} {fe:8.4f} {mism:9d}", flush=True)

if __name__ == "__main__":
    main(sys.argv[1:])
