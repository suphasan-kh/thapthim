# test/eval_segment.rb
#
# Word-level segmentation evaluation harness for Thapthim.
#
# Reports micro-averaged word-span F1 (a predicted word is correct iff BOTH of its
# boundaries match the gold word) plus throughput, for any gold corpus stored as a
# JSON array-of-arrays (each inner array = one sentence's gold token list).
#
# Gold corpora live in datasets/ (separate from the shipped model in ext/thapthim/assets/):
#   - tnhc_test.json         (TNHC literary, our dev/anchor set)
#   - LST20_test_cleaned.json
#   - BEST_train_cleaned.json
#
# Usage:
#   ruby test/eval_segment.rb                 # eval every known corpus
#   ruby test/eval_segment.rb tnhc lst20      # eval a subset by short name
#   ruby test/eval_segment.rb /path/file.json # eval an explicit corpus file
#
# Env:
#   THAPTHIM_EVAL_LIMIT=2000   # cap sentences (handy for the 79MB BEST file)
#   THAPTHIM_EVAL_WARM=1       # run a warm-up pass before timing (steadier speed)
#
require "json"
require "set"
require_relative "../lib/thapthim"

module SegEval
  DATASET_DIR = File.expand_path("../datasets", __dir__)

  # short name => filename
  CORPORA = {
    "tnhc"   => "tnhc_test.json",
    "lst20"  => "LST20_test_cleaned.json",
    "best"   => "BEST_train_cleaned.json",
    "vistec" => "VISTEC_test.json",
  }.freeze

  # Turn a token list into its set of [start, end) character spans. Tokens tile the
  # sentence exactly, so cumulative char-length gives every word's offset. When
  # +exclude_space+ is set, whitespace-only tokens are dropped from the set AFTER
  # offsets are computed (so the remaining spans stay aligned to the real text).
  def self.spans(tokens, exclude_space)
    out = Set.new
    pos = 0
    tokens.each do |tok|
      len = tok.length
      out << [pos, pos + len] unless exclude_space && tok.strip.empty?
      pos += len
    end
    out
  end

  Result = Struct.new(
    :name, :sentences, :chars, :seconds,
    :tp, :fp, :fn, :tp_ns, :fp_ns, :fn_ns, :mismatches,
    keyword_init: true
  ) do
    def precision(tp, fp) = tp + fp == 0 ? 0.0 : tp.to_f / (tp + fp)
    def recall(tp, fn)    = tp + fn == 0 ? 0.0 : tp.to_f / (tp + fn)
    def f1(tp, fp, fn)
      p = precision(tp, fp)
      r = recall(tp, fn)
      (p + r).zero? ? 0.0 : 2 * p * r / (p + r)
    end

    def report
      sps = sentences / seconds
      cps = chars / seconds
      <<~TXT
        #{name}
          sentences      : #{sentences}  (#{mismatches} reconstruction mismatch#{mismatches == 1 ? "" : "es"})
          incl. spaces   : P=#{format("%.4f", precision(tp, fp))}  R=#{format("%.4f", recall(tp, fn))}  F1=#{format("%.4f", f1(tp, fp, fn))}
          excl. spaces   : P=#{format("%.4f", precision(tp_ns, fp_ns))}  R=#{format("%.4f", recall(tp_ns, fn_ns))}  F1=#{format("%.4f", f1(tp_ns, fp_ns, fn_ns))}
          speed          : #{format("%.0f", sps)} sent/s  |  #{format("%.0f", cps)} char/s  |  #{format("%.2f", seconds)}s total
      TXT
    end
  end

  def self.evaluate(path, name: File.basename(path), limit: nil, warm: false)
    sentences = JSON.parse(File.read(path))
    sentences = sentences.first(limit) if limit

    if warm
      sentences.first([sentences.size, 200].min).each { |t| Thapthim.segment(t.join) }
    end

    tp = fp = fn = 0
    tp_ns = fp_ns = fn_ns = 0
    chars = 0
    mismatches = 0

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    sentences.each do |gold_tokens|
      text = gold_tokens.join
      chars += text.length
      pred_tokens = Thapthim.segment(text)
      mismatches += 1 unless pred_tokens.join == text

      g  = spans(gold_tokens, false)
      pr = spans(pred_tokens, false)
      hit = (g & pr).size
      tp += hit
      fp += pr.size - hit
      fn += g.size - hit

      gn  = spans(gold_tokens, true)
      prn = spans(pred_tokens, true)
      hitn = (gn & prn).size
      tp_ns += hitn
      fp_ns += prn.size - hitn
      fn_ns += gn.size - hitn
    end
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    Result.new(
      name: name, sentences: sentences.size, chars: chars, seconds: elapsed,
      tp: tp, fp: fp, fn: fn, tp_ns: tp_ns, fp_ns: fp_ns, fn_ns: fn_ns,
      mismatches: mismatches
    )
  end

  # Resolve a CLI argument (short name or explicit path) to [name, path].
  def self.resolve(arg)
    if CORPORA.key?(arg)
      [arg, File.join(DATASET_DIR, CORPORA[arg])]
    elsif File.exist?(arg)
      [File.basename(arg, ".json"), arg]
    else
      warn "skip: no corpus '#{arg}' (known: #{CORPORA.keys.join(", ")} or a JSON path)"
      nil
    end
  end

  def self.main(argv)
    limit = ENV["THAPTHIM_EVAL_LIMIT"]&.to_i
    warm  = ENV["THAPTHIM_EVAL_WARM"] == "1"

    targets = argv.empty? ? CORPORA.keys : argv
    specs = targets.filter_map { |a| resolve(a) }.select { |_, path| File.exist?(path) }

    if specs.empty?
      warn "no evaluatable corpora found in #{DATASET_DIR}"
      exit 1
    end

    puts "Thapthim word-segmentation eval  (limit=#{limit || "none"}, warm=#{warm})"
    puts "metric: micro-averaged word-span F1 (both boundaries must match)\n\n"
    specs.each do |name, path|
      puts SegEval.evaluate(path, name: name, limit: limit, warm: warm).report
    end
  end
end

SegEval.main(ARGV) if __FILE__ == $PROGRAM_NAME
