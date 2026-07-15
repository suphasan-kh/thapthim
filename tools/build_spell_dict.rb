# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# tools/build_spell_dict.rb
# Builds the spelling-suggestion dictionary asset from PyThaiNLP's thai_words() list.
# Regenerate the raw dump in an env with pythainlp:
#   python -c "from pythainlp.corpus import thai_words; \
#              open('thai_words_raw.txt','w').write('\n'.join(sorted(thai_words())))"
# then:  ruby -Ilib tools/build_spell_dict.rb thai_words_raw.txt ext/thapthim/assets/spell_words.txt
#
# Cleaning (a spelling dict must hold ONLY well-formed single words, unlike the permissive
# segmentation dict): keep entries that are ≥2 characters and purely Thai letters/vowels/tones
# (U+0E01–U+0E4E). This drops single consonants (ฆ ฏ ฒ), whitespace phrases, period abbreviations
# (ก.พ.), and numbered named entities. Each survivor is std_normalized (canonical form matching
# runtime input) and de-duplicated.
require_relative "../lib/thapthim"

raw_path = ARGV[0] or abort "usage: build_spell_dict.rb <raw_wordlist> <output_asset>"
out_path = ARGV[1] or abort "usage: build_spell_dict.rb <raw_wordlist> <output_asset>"

THAI_WORD = /\A[ก-๎]+\z/ # Thai letters/vowels/tones/signs only (no digits, spaces, ASCII)

raw = File.readlines(raw_path, chomp: true)
kept = raw.filter_map do |word|
  word = word.strip
  next if word.length < 2 || !word.match?(THAI_WORD)

  Thapthim.std_normalize(word)
end
kept.uniq!
kept.sort!

File.write(out_path, kept.join("\n") + "\n")
puts "read #{raw.size} raw entries → wrote #{kept.size} spelling words to #{out_path}"
puts "dropped: #{raw.size - kept.size} (single chars, phrases, abbreviations, numbered entities, dups)"
