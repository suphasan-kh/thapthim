# test/eval_correct.rb
#
# Evaluation harness for the Thapthim lexical spelling corrector.
#
# Two metrics, two corpus shapes (auto-detected per file by peeking the first line):
#
#   NO-HARM  (clean corpora: a JSONL of gold token arrays, e.g. lst20/best/tnhc)
#     Run the corrector on already-correct text and measure how often it CHANGES a word.
#     This is the make-or-break guardrail — altering correct text is the expensive error
#     (the direct analogue of "don't break R_iv" from segmentation). Target: << 1%.
#     An identity corrector must score exactly 0.00%, which is how this harness is validated
#     before any real correction logic exists.
#
#   CORRECTION  (typo corpora: a JSONL of {"err":[...tokens], "cor":[...tokens]} pairs,
#               e.g. VISTEC-TP-TH-2021 — NOT yet in datasets/, see plan deliverable 0)
#     Sentence-level exact-correction accuracy: fraction of erroneous sentences whose
#     corrected output matches the gold correction (after identical normalization of both
#     sides, so normalization differences are never counted as correction errors).
#     Word-level span precision/recall will be added on top of the annotation API in Phase 1.
#
# The corrector is resolved at load time: Thapthim.correct if the native binding exists,
# otherwise an identity fallback (std_normalize only) so the harness runs and the no-harm
# baseline can be validated today.
#
# Usage:
#   ruby test/eval_correct.rb                       # no-harm on clean corpora (lst20 best tnhc)
#   ruby test/eval_correct.rb tnhc                  # subset
#   ruby test/eval_correct.rb /path/vistec_tp.jsonl # correction eval on a typo corpus
#
# Env:
#   THAPTHIM_EVAL_LIMIT=2000   # cap sentences
#
require "json"
require "set"
require_relative "eval_segment"

module CorrectEval
  CLEAN_DEFAULTS = %w[lst20 best tnhc].freeze

  # Resolve the corrector once. Identity fallback = std_normalize only (changes no words),
  # which makes the no-harm baseline exactly 0% and validates the harness pre-implementation.
  def self.corrector
    @corrector ||=
      if Thapthim.respond_to?(:correct)
        ->(text) { Thapthim.correct(text) }
      else
        warn "[eval_correct] Thapthim.correct not found — using identity fallback (std_normalize)"
        ->(text) { Thapthim.std_normalize(text.to_s) }
      end
  end

  # Number of word tokens present in +before+ but not matched in +after+ (multiset diff).
  # 0 when the two segmentations are identical.
  def self.changed_tokens(before, after)
    after_tally = after.tally
    removed = 0
    before.tally.each do |tok, n|
      removed += [n - after_tally.fetch(tok, 0), 0].max
    end
    removed
  end

  NoHarm = Struct.new(:name, :sentences, :changed_sentences, :words, :changed_words, keyword_init: true) do
    def sentence_rate = sentences.zero? ? 0.0 : changed_sentences.to_f / sentences
    def word_rate     = words.zero? ? 0.0 : changed_words.to_f / words

    def report
      <<~TXT
        #{name}  (no-harm on clean text)
          sentences      : #{sentences}
          words altered  : #{changed_words}/#{words}  (#{format("%.3f", word_rate * 100)}%)   <- guardrail, want <<1%
          sents altered  : #{changed_sentences}/#{sentences}  (#{format("%.3f", sentence_rate * 100)}%)
      TXT
    end
  end

  def self.no_harm(path, name:, limit:)
    sentences = SegEval.read_jsonl(path, limit)
    changed_sentences = words = changed_words = 0

    sentences.each do |gold_tokens|
      text = gold_tokens.join
      norm = Thapthim.std_normalize(text)
      corr = corrector.call(text)
      seg_norm = Thapthim.word_segment(norm)
      words += seg_norm.length
      next if corr == norm # fast path: nothing changed

      changed_sentences += 1
      changed_words += changed_tokens(seg_norm, Thapthim.word_segment(corr))
    end

    NoHarm.new(name: name, sentences: sentences.size, changed_sentences: changed_sentences,
               words: words, changed_words: changed_words)
  end

  Corr = Struct.new(:name, :pairs, :erroneous, :fixed, keyword_init: true) do
    # accuracy over erroneous sentences only (sentences already correct are excluded so the
    # number reflects real correction skill, not the corpus's clean-sentence fraction)
    def accuracy = erroneous.zero? ? 0.0 : fixed.to_f / erroneous

    def report
      <<~TXT
        #{name}  (sentence-level correction)
          pairs          : #{pairs}  (#{erroneous} actually erroneous after normalization)
          fixed exactly  : #{fixed}/#{erroneous}  (acc=#{format("%.4f", accuracy)} over erroneous)
      TXT
    end
  end

  def self.correction(path, name:, limit:)
    rows = SegEval.read_jsonl(path, limit)
    erroneous = fixed = 0

    rows.each do |row|
      err_n = Thapthim.std_normalize(row["err"].join)
      cor_n = Thapthim.std_normalize(row["cor"].join)
      next if err_n == cor_n # no error to fix after normalization

      erroneous += 1
      pred = corrector.call(row["err"].join)
      fixed += 1 if pred == cor_n
    end

    Corr.new(name: name, pairs: rows.size, erroneous: erroneous, fixed: fixed)
  end

  # Peek the first non-empty line to classify the corpus: a JSON object with err/cor keys is a
  # typo corpus; a JSON array is a clean gold corpus for the no-harm metric.
  def self.typo_corpus?(path)
    File.foreach(path) do |line|
      next if line.strip.empty?
      return JSON.parse(line).is_a?(Hash)
    end
    false
  end

  def self.main(argv)
    limit = ENV["THAPTHIM_EVAL_LIMIT"]&.to_i
    targets = argv.empty? ? CLEAN_DEFAULTS : argv
    specs = targets.filter_map { |a| SegEval.resolve(a) }.select { |_, path| File.exist?(path) }
    if specs.empty?
      warn "no evaluatable corpora found"
      exit 1
    end

    puts "Thapthim spelling-correction eval  (limit=#{limit || "none"})"
    puts "corrector: #{Thapthim.respond_to?(:correct) ? "native Thapthim.correct" : "IDENTITY fallback (std_normalize)"}\n\n"
    specs.each do |name, path|
      result = typo_corpus?(path) ? correction(path, name: name, limit: limit)
                                  : no_harm(path, name: name, limit: limit)
      puts result.report
    end
  end
end

CorrectEval.main(ARGV) if __FILE__ == $PROGRAM_NAME
