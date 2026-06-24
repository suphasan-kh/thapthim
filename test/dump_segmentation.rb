# test/dump_segmentation.rb
#
# Dump Thapthim's word segmentation for each benchmark corpus to JSON Lines, one predicted
# token-array per gold sentence, so the Python accuracy harness (test/benchmark_accuracy.py)
# can score Thapthim with the EXACT same metric it uses for attacut/deepcut/nlpo3/newmm — the
# pythainlp.benchmarks reference metric. Keeping prediction generation (Ruby/Rust) separate
# from scoring (Python) is what lets every engine go through one identical scorer.
#
# Usage:
#   ruby test/dump_segmentation.rb <out_dir>            # default (shipped LST20) LM
#   THAPTHIM_LM=best ruby test/dump_segmentation.rb <out_dir>   # gated BEST LM (needs best_lm build)
#
# The corpus list and per-corpus caps MUST match CORPORA in test/benchmark_accuracy.py so the
# predictions line up with the gold sentences the scorer reads.
require "json"
require "fileutils"
require_relative "../lib/thapthim"

DATASETS = File.expand_path("../datasets", __dir__)

# short => [filename, limit]   (caps bound deepcut's runtime; see benchmark_accuracy.py)
CORPORA = {
  "lst20"  => ["LST20_test_cleaned.jsonl", 5250],
  "best"   => ["BEST_test_cleaned.jsonl",  3000],
  "vistec" => ["VISTEC_test.jsonl",        3000],
  "tnhc"   => ["tnhc_test.jsonl",          4403],
  "ws1000" => ["ws1000.jsonl",             993],
}.freeze

out = ARGV[0] or abort "usage: ruby test/dump_segmentation.rb <out_dir>"
FileUtils.mkdir_p(out)

# LIMIT env overrides every per-corpus cap (use a large value for a full-size run). Must match
# the LIMIT passed to test/benchmark_accuracy.py so the scorer reads the same sentence set.
limit_override = ENV["LIMIT"]&.to_i

CORPORA.each do |short, (fname, cap)|
  lim = limit_override || cap
  path = File.join(DATASETS, fname)
  File.open(File.join(out, "#{short}.jsonl"), "w") do |f|
    File.foreach(path).first(lim).each do |line|
      next if line.strip.empty?
      text = JSON.parse(line).join
      f.puts JSON.generate(Thapthim.segment(text))
    end
  end
  warn "  #{short}: #{[File.foreach(path).count, lim].min} sentences"
end
warn "dumped Thapthim predictions to #{out} (LM=#{ENV.fetch('THAPTHIM_LM', 'default/LST20')})"
