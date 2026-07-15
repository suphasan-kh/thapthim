# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# tools/build_spell_unigrams.rb
# Builds a `thai_words`-ALIGNED unigram frequency table for spelling-correction ranking.
#
# Why: the shipped word LM (kn_words_unigrams.txt) is trained on LST20's gold segmentation, whose
# word inventory differs from the spelling dictionary (thai_words). Many thai_words entries — e.g.
# ความสามารถ — are absent from that LM (LST20 splits them), so correction ranking gets no frequency
# signal for them. This tool re-segments a corpus by MAXIMAL MATCH against thai_words (so those words
# stay whole) and counts unigram frequencies over that tokenization.
#
# NOTE this table is "contaminated" relative to a clean dictionary: the counts reflect maximal-match
# segmentation errors and the corpus content. It is a *ranking* source only — the candidate dictionary
# (spell_words.txt) stays clean. The corrector selects between this and the LST20 table at runtime.
#
# Usage:
#   ruby tools/build_spell_unigrams.rb ext/thapthim/assets/spell_words.txt \
#        ext/thapthim/assets/spell_unigrams_aligned.txt datasets/LST20_train_cleaned.jsonl datasets/BEST_train_cleaned.jsonl
require "json"
require "set"

dict_path, out_path, *corpora = ARGV
abort "usage: build_spell_unigrams.rb <dict> <out> <corpus.jsonl...>" if !dict_path || !out_path || corpora.empty?

words = File.readlines(dict_path, chomp: true).reject(&:empty?)
DICT = words.to_set
MAX_MATCH = 40 # cap match length; thai_words entries longer than this are negligible and slow to probe
LENGTHS = words.map(&:length).select { |l| l <= MAX_MATCH }.uniq.sort.reverse.freeze

# Longest-match tokenization against the dictionary; unmatched characters become 1-char fragments.
def maxmatch(text, lengths)
  chars = text.chars
  n = chars.length
  i = 0
  out = []
  while i < n
    hit = nil
    lengths.each do |len|
      break if i + len > n && len > n - i # lengths are descending; only skip too-long
      next if i + len > n
      w = chars[i, len].join
      if DICT.include?(w)
        hit = w
        i += len
        break
      end
    end
    if hit
      out << hit
    else
      out << chars[i]
      i += 1
    end
  end
  out
end

freq = Hash.new(0)
lines = 0
corpora.each do |path|
  next unless File.exist?(path)

  File.foreach(path) do |line|
    toks = (JSON.parse(line) rescue next)
    maxmatch(toks.join, LENGTHS).each { |w| freq[w] += 1 if DICT.include?(w) }
    lines += 1
  end
end

# Emit only dictionary words actually seen (unseen words fall to the floor at lookup time).
File.open(out_path, "w") do |f|
  freq.select { |w, _| DICT.include?(w) }.sort_by { |_, c| -c }.each { |w, c| f.puts "#{w}\t#{c}" }
end
puts "segmented #{lines} lines -> #{freq.count { |w, _| DICT.include?(w) }} dict words with counts -> #{out_path}"
