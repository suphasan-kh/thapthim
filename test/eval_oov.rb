# test/eval_oov.rb
#
# OOV-stratified recall harness for Thapthim (SIGHAN-style R_iv / R_oov).
#
# The plain word-span F1 in eval_segment.rb hides whether the model only recalls
# words it already has in its dictionary. This script splits every gold word into
# two buckets by dictionary membership and reports recall in each:
#
#   R_iv   recall on in-vocabulary gold words   (memorization)
#   R_oov  recall on out-of-vocabulary words    (generalization — the branching-
#          entropy merge post-pass is what's meant to move this number)
#
# "In vocabulary" = present in the shipped word lexicon that the engine embeds:
#   ext/thapthim/assets/master_words_vocab.txt
# For BEST the held-out test split is decontaminated from that lexicon (see
# eval_segment.rb), so BEST_test OOV words are genuinely unseen.
#
# A gold word counts as recalled iff its exact [start, end) character span appears
# in the prediction. Whitespace-only tokens are excluded from both buckets.
#
# Usage:
#   ruby test/eval_oov.rb                  # eval every known corpus
#   ruby test/eval_oov.rb tnhc lst20       # subset by short name
#   ruby test/eval_oov.rb /path/file.jsonl # explicit corpus file
#
# Env:
#   THAPTHIM_EVAL_LIMIT=2000   # cap sentences (handy for the big BEST/VISTEC files)
#   THAPTHIM_VOCAB=/path.txt   # override the in-vocab lexicon
#
require "json"
require "set"
require_relative "eval_segment"

module OovEval
  DEFAULT_VOCAB = File.expand_path("../ext/thapthim/assets/master_words_vocab.txt", __dir__)

  def self.load_vocab(path)
    vocab = Set.new
    File.foreach(path) do |line|
      w = line.chomp
      vocab << w unless w.empty?
    end
    vocab
  end

  Result = Struct.new(
    :name, :sentences, :iv_total, :iv_hit, :oov_total, :oov_hit,
    keyword_init: true
  ) do
    def recall(hit, total) = total.zero? ? 0.0 : hit.to_f / total
    def oov_rate           = (iv_total + oov_total).zero? ? 0.0 : oov_total.to_f / (iv_total + oov_total)

    def report
      <<~TXT
        #{name}
          words          : #{iv_total + oov_total}  (#{format("%.1f", oov_rate * 100)}% OOV vs dictionary)
          R_iv           : #{format("%.4f", recall(iv_hit, iv_total))}   (#{iv_hit}/#{iv_total} in-vocab words recalled)
          R_oov          : #{format("%.4f", recall(oov_hit, oov_total))}   (#{oov_hit}/#{oov_total} OOV words recalled)
      TXT
    end
  end

  # Yield [word, start, end] for each non-whitespace token, with char offsets that
  # stay aligned to the real text (offsets advance through spaces; spaces are only
  # skipped from being yielded). Mirrors SegEval.spans' offset accounting.
  def self.each_word_span(tokens)
    pos = 0
    tokens.each do |tok|
      len = tok.length
      yield tok, pos, pos + len unless tok.strip.empty?
      pos += len
    end
  end

  def self.evaluate(path, vocab, name: File.basename(path), limit: nil)
    sentences = SegEval.read_jsonl(path, limit)
    iv_total = iv_hit = oov_total = oov_hit = 0

    sentences.each do |gold_tokens|
      text = gold_tokens.join
      pred_spans = SegEval.spans(Thapthim.segment(text), true)

      each_word_span(gold_tokens) do |word, s, e|
        recalled = pred_spans.include?([s, e])
        if vocab.include?(word)
          iv_total += 1
          iv_hit += 1 if recalled
        else
          oov_total += 1
          oov_hit += 1 if recalled
        end
      end
    end

    Result.new(
      name: name, sentences: sentences.size,
      iv_total: iv_total, iv_hit: iv_hit, oov_total: oov_total, oov_hit: oov_hit
    )
  end

  def self.main(argv)
    limit = ENV["THAPTHIM_EVAL_LIMIT"]&.to_i
    vocab_path = ENV["THAPTHIM_VOCAB"] || DEFAULT_VOCAB
    unless File.exist?(vocab_path)
      warn "no vocab file at #{vocab_path} (set THAPTHIM_VOCAB)"
      exit 1
    end
    vocab = load_vocab(vocab_path)

    targets = argv.empty? ? SegEval::CORPORA.keys : argv
    specs = targets.filter_map { |a| SegEval.resolve(a) }.select { |_, path| File.exist?(path) }
    if specs.empty?
      warn "no evaluatable corpora found"
      exit 1
    end

    puts "Thapthim OOV-stratified recall  (limit=#{limit || "none"})"
    puts "in-vocab = #{File.basename(vocab_path)} (#{vocab.size} words)"
    puts "metric: word-span recall, gold words split by dictionary membership\n\n"
    specs.each do |name, path|
      puts evaluate(path, vocab, name: name, limit: limit).report
    end
  end
end

OovEval.main(ARGV) if __FILE__ == $PROGRAM_NAME
