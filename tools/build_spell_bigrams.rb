# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# tools/build_spell_bigrams.rb
# Builds a thai_words-ALIGNED bigram table for context-aware correction (correct_sent). Companion to
# build_spell_unigrams.rb: same maximal-match-to-thai_words re-segmentation, but counts adjacent word
# pairs. The shipped LST20 bigram LM lacks many thai_words words (it splits e.g. ความสามารถ), so its
# context scoring is blind to them; this table gives correct_sent a bigram over the dict vocabulary.
# Corpus-derived ("contaminated"), a scoring source only — the candidate dictionary stays clean.
#
# Singletons are pruned (count >= MIN) to keep the asset small; the model backs off to the aligned
# unigram for unseen pairs anyway. Format: "<w1>\t<w2>\t<count>".
#
# Usage:
#   ruby tools/build_spell_bigrams.rb ext/thapthim/assets/spell_words.txt \
#        ext/thapthim/assets/spell_bigrams_aligned.txt datasets/LST20_train_cleaned.jsonl datasets/BEST_train_cleaned.jsonl
require "json"
require "set"

dict_path, out_path, *corpora = ARGV
abort "usage: build_spell_bigrams.rb <dict> <out> <corpus.jsonl...>" if !dict_path || !out_path || corpora.empty?
MIN = (ENV["MIN"] || 2).to_i

words = File.readlines(dict_path, chomp: true).reject(&:empty?)
DICT = words.to_set
MAX_MATCH = 40
LENGTHS = words.map(&:length).select { |l| l <= MAX_MATCH }.uniq.sort.reverse.freeze

def maxmatch(text, lengths)
  chars = text.chars
  n = chars.length
  i = 0
  out = []
  while i < n
    hit = nil
    lengths.each do |len|
      next if i + len > n
      w = chars[i, len].join
      if DICT.include?(w)
        hit = w
        i += len
        break
      end
    end
    if hit then out << hit else out << chars[i]; i += 1 end
  end
  out
end

bigrams = Hash.new(0)
lines = 0
corpora.each do |path|
  next unless File.exist?(path)

  File.foreach(path) do |line|
    toks = (JSON.parse(line) rescue next)
    seq = maxmatch(toks.join, LENGTHS).select { |w| DICT.include?(w) }
    seq.each_cons(2) { |a, b| bigrams[[a, b]] += 1 }
    lines += 1
  end
end

kept = 0
File.open(out_path, "w") do |f|
  bigrams.select { |_, c| c >= MIN }.sort_by { |_, c| -c }.each do |(a, b), c|
    f.puts "#{a}\t#{b}\t#{c}"
    kept += 1
  end
end
puts "segmented #{lines} lines -> #{bigrams.size} bigram types, kept #{kept} (count>=#{MIN}) -> #{out_path}"
