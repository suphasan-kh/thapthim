# test/benchmark_speed.py
#
# Throughput benchmark for the Python (PyO3) binding, using the SAME protocol as the Ruby
# benchmark (test/benchmark_speed.rb): same corpus, warmup excluded, best-of-N repetitions,
# char/s + sent/s. Run the two side by side to compare binding overhead:
#
#   ruby                          test/benchmark_speed.rb [N] [REPS]
#   PYTHONPATH=<dir-with-thapthim.so> python3 test/benchmark_speed.py [N] [REPS]
#
# The extra `segment_batch` line exercises the PyO3-only perf lever (one boundary crossing, GIL
# released, rayon across cores) and has no Ruby equivalent.
import json, os, sys, time
import thapthim as t

CORPUS = os.path.join(os.path.dirname(__file__), "..", "datasets", "LST20_test_cleaned.jsonl")

n = int(sys.argv[1]) if len(sys.argv) > 1 else 3000
reps = int(sys.argv[2]) if len(sys.argv) > 2 else 3

with open(CORPUS, encoding="utf-8") as f:
    texts = ["".join(json.loads(line)) for line in list(f)[:n]]
chars = sum(len(s) for s in texts)

for s in texts[:100]:  # warmup (untimed)
    t.word_segment(s)

# Per-call loop (directly comparable to the Ruby benchmark).
loop_times = []
for _ in range(reps):
    t0 = time.perf_counter()
    for s in texts:
        t.word_segment(s)
    loop_times.append(time.perf_counter() - t0)
best, mean = min(loop_times), sum(loop_times) / len(loop_times)
print("thapthim-py  n=%d chars=%d reps=%d | best=%9.0f char/s  mean=%9.0f char/s  (%6.0f sent/s)"
      % (len(texts), chars, reps, chars / best, chars / mean, len(texts) / best))

# Batch lever (PyO3 only): amortized boundary crossing + rayon multicore.
batch_times = []
for _ in range(reps):
    t0 = time.perf_counter()
    t.word_segment_batch(texts)
    batch_times.append(time.perf_counter() - t0)
bbest = min(batch_times)
print("thapthim-py  segment_batch                | best=%9.0f char/s  (%6.0f sent/s)  [GIL-released, multicore]"
      % (chars / bbest, len(texts) / bbest))
