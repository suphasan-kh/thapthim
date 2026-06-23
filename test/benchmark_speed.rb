# test/benchmark_speed.rb
#
# Controlled, warm throughput benchmark for Thapthim.segment, using the same protocol as the
# Python baseline bench (warmup excluded, fixed corpus, best-of-N repetitions). Run:
#   ruby test/benchmark_speed.rb [N_SENTENCES] [REPS]
require "json"
require_relative "../lib/thapthim"

CORPUS = File.expand_path("../datasets/LST20_test_cleaned.json", __dir__)

n    = (ARGV[0] || 3000).to_i
reps = (ARGV[1] || 3).to_i

texts = JSON.parse(File.read(CORPUS)).first(n).map(&:join)
chars = texts.sum(&:length)

texts.first(100).each { |t| Thapthim.segment(t) } # warmup (untimed)

times = Array.new(reps) do
  t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  texts.each { |t| Thapthim.segment(t) }
  Process.clock_gettime(Process::CLOCK_MONOTONIC) - t0
end

best = times.min
mean = times.sum / times.size
printf("thapthim  n=%d chars=%d reps=%d | best=%9.0f char/s  mean=%9.0f char/s  (%6.0f sent/s)\n",
       texts.size, chars, reps, chars / best, chars / mean, texts.size / best)
