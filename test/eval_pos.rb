# test/eval_pos.rb
#
# POS-tagging evaluation harness for Thapthim, run through the public Ruby API (Thapthim.pos_tag on
# GOLD tokens — array input — so a segmentation error can never contaminate the number).
#
# Reports token accuracy overall AND split by known vs OOV word. The split is the point: a single
# blended figure hides that the HMM is strong on seen vocabulary and weak on the OOV tail (which is
# where a future suffix model / CRF would pay off, and which balloons off-domain). Also prints the
# worst confusions so the errors are legible.
#
# The corpus is the full tagged LST20 (4-column `token \t POS \t NER \t clause`, blank line = sentence
# boundary), NOT the segmentation-only jsonl. Known-word membership is built from the train split's
# token column, canonicalized the same way the model was (convention 1: `_`/whitespace -> space).
#
# Usage:
#   ruby test/eval_pos.rb datasets/LST20_full_train datasets/LST20_full_test
#
# Uses the embedded model by default; set THAPTHIM_POS_MODEL=<path> to evaluate a work-in-progress
# model instead (e.g. one the trainer just wrote elsewhere).
require_relative "../lib/thapthim"

module PosEval
  # Convention (1): the LST20 space token `_`, and any whitespace-only surface, collapse to a single
  # space so "space" is one consistent vocabulary entry — must match `canon_surface` in the Rust core.
  def self.canon(surface)
    return " " if surface == "_" || (!surface.empty? && surface.match?(/\A\s+\z/))
    surface
  end

  # Set of canonical surfaces seen in the training split — the known/OOV oracle for the accuracy split.
  def self.known_vocab(train_dir)
    known = {}
    Dir.glob(File.join(train_dir, "*.txt")).each do |file|
      File.foreach(file) do |line|
        line = line.chomp
        next if line.empty?
        tok = line.split("\t", 2).first
        known[canon(tok)] = true if tok
      end
    end
    known
  end

  # Parse an LST20 .txt directory into [words, gold_tags] sentence pairs (surfaces canonicalized).
  def self.read_sentences(test_dir)
    sentences = []
    words = []
    tags = []
    flush = -> { (sentences << [words, tags]; words = []; tags = []) unless words.empty? }

    Dir.glob(File.join(test_dir, "*.txt")).each do |file|
      File.foreach(file) do |line|
        line = line.chomp
        if line.empty?
          flush.call
          next
        end
        cols = line.split("\t")
        next unless cols[0] && cols[1]
        words << canon(cols[0])
        tags << cols[1]
      end
      flush.call # final sentence if the file lacks a trailing blank line
    end
    sentences
  end

  def self.run(train_dir, test_dir)
    warn "building known-word set from #{train_dir} ..."
    known = known_vocab(train_dir)
    warn "reading test sentences from #{test_dir} ..."
    sentences = read_sentences(test_dir)

    total = correct = 0
    known_n = known_ok = 0
    oov_n = oov_ok = 0
    confusion = Hash.new(0) # [gold, pred] => count, for mismatches only

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    sentences.each do |words, gold|
      pred = Thapthim.pos_tag(words).map(&:last)
      words.each_index do |i|
        ok = pred[i] == gold[i]
        total += 1
        correct += 1 if ok
        confusion[[gold[i], pred[i]]] += 1 unless ok
        if known[canon(words[i])]
          known_n += 1
          known_ok += 1 if ok
        else
          oov_n += 1
          oov_ok += 1 if ok
        end
      end
    end
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    pct = ->(ok, n) { n.zero? ? 0.0 : 100.0 * ok / n }
    puts "sentences: #{sentences.length}"
    puts "tokens:    #{total}"
    puts format("overall accuracy: %.2f%%  (%d/%d)", pct.call(correct, total), correct, total)
    puts format("  known-word:  %.2f%%  (%d/%d)", pct.call(known_ok, known_n), known_ok, known_n)
    puts format("  OOV-word:    %.2f%%  (%d/%d,  %.2f%% of tokens)",
                pct.call(oov_ok, oov_n), oov_ok, oov_n, pct.call(oov_n, total))
    puts format("throughput:  %.0f tokens/s", total / elapsed)

    puts "top confusions (gold -> pred : count):"
    confusion.sort_by { |_, c| -c }.first(10).each do |(gold, pred), c|
      puts "  #{gold} -> #{pred} : #{c}"
    end
  end
end

train_dir = ARGV[0] || File.expand_path("../datasets/LST20_full_train", __dir__)
test_dir  = ARGV[1] || File.expand_path("../datasets/LST20_full_test", __dir__)
PosEval.run(train_dir, test_dir)
