import json, time, sys, os, itertools

DATASETS = "/Users/louis/Documents/Development/Ruby/thapthim/datasets"
CORPUS = os.path.join(DATASETS, "LST20_test_cleaned.jsonl")

def load(n):
    with open(CORPUS) as f:
        sents = [json.loads(line) for line in itertools.islice(f, n)]
    texts = ["".join(s) for s in sents]
    chars = sum(len(t) for t in texts)
    return texts, chars

def bench(tok, texts, chars, reps):
    # warmup (untimed): load model, JIT, fill caches
    for t in texts[:100]:
        tok(t)
    times = []
    for _ in range(reps):
        t0 = time.perf_counter()
        for t in texts:
            tok(t)
        times.append(time.perf_counter() - t0)
    best, mean = min(times), sum(times) / len(times)
    return chars / best, chars / mean, len(texts) / best

def main():
    engine, n, reps = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    if engine == "newmm":
        from pythainlp.tokenize import word_tokenize
        tok = lambda s: word_tokenize(s, engine="newmm")
    elif engine == "attacut":
        from attacut import Tokenizer
        t = Tokenizer(model="attacut-sc")
        tok = t.tokenize
    elif engine == "deepcut":
        import deepcut
        tok = deepcut.tokenize
    else:
        raise SystemExit("unknown")
    texts, chars = load(n)
    best_cps, mean_cps, best_sps = bench(tok, texts, chars, reps)
    print(f"{engine:9s} n={len(texts)} chars={chars} reps={reps} | "
          f"best={best_cps:9.0f} char/s  mean={mean_cps:9.0f} char/s  ({best_sps:6.0f} sent/s)")

main()
