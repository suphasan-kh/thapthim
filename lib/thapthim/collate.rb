# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# frozen_string_literal: true

module Thapthim
  # EXPERIMENTAL — NOT YET VERIFIED. The unit tests pass, but this has not been validated against the
  # dictionary at scale or reviewed by the author; treat the ordering as provisional until then.
  #
  # Thai dictionary collation, following the Royal Society of Thailand (ราชบัณฑิตยสภา) ordering
  # (rules: https://th.wikibooks.org/wiki/วิธีเรียงลำดับคำตามตัวอักษรในภาษาไทย — encoded in test_collate.rb).
  # Native Ruby sorts Thai by codepoint, which is wrong: the pre-posed vowels เ แ โ ใ ไ are written
  # before their consonant but sort after it, so codepoint order scatters เก/ไก words away from their
  # base consonant. This module fixes that with two rules — (1) reorder a leading vowel after the
  # consonant it precedes, and (2) treat tone marks as a secondary level so ปา < ป่า < ป้า.
  #
  # Pure Ruby, no engine dependency. Where to use it BEYOND a one-off sort:
  #   * Any Enumerable ordering of Thai text — sort_by / min_by / max_by / Enumerable#sort — e.g.
  #     dropdowns, autocomplete, contact and product lists, or A–Z index/glossary pages.
  #   * A database sort column: store `thai_sort_key(name)` in an indexed column and `ORDER BY` it,
  #     so SQL returns Thai in dictionary order (DB collations for Thai are usually missing or wrong)
  #     — and it composes with pagination. The key is an opaque, comparable binary String.

  # Consonants ก–ฮ in alphabet order (ฤ placed after ร, ฦ after ล, as the dictionary does).
  COLLATION_CONSONANTS = "กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรฤลฦวศษสหฬอฮ"
  # Primary sort units, in dictionary order. Mostly single characters, but ฤๅ and ฦๅ are their own
  # letters — ordered immediately after ฤ and ฦ (…ร ฤ ฤๅ ล ฦ ฦๅ ว…) — so they are matched as a
  # two-character digraph before their first character is read alone. Then the vowel signs; a bare ๅ
  # (not part of ฤๅ/ฦๅ) ranks last.
  COLLATION_PRIMARY_UNITS = (
    "กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยร".chars +
    %w[ฤ ฤๅ ล ฦ ฦๅ] +
    "วศษสหฬอฮ".chars +
    "ะัาำิีึืุูเแโใไ".chars + %w[ๅ]
  ).freeze
  # Secondary (compared only after the primary letters tie): maitaikhu, the four tone marks, then
  # thanthakhat (การันต์) and nikhahit. "No mark" ranks 0, below all of these.
  COLLATION_SECONDARY = "็่้๊๋์ํ"
  # The pre-posed (leading) vowels that must be reordered after their consonant.
  COLLATION_LEADING_VOWELS = "เแโใไ"

  PRIMARY_RANK   = COLLATION_PRIMARY_UNITS.each_with_index.to_h { |u, i| [u, i + 1] }.freeze
  SECONDARY_RANK = COLLATION_SECONDARY.chars.each_with_index.to_h { |c, i| [c, i + 1] }.freeze

  # Sort Thai strings by dictionary order. Without a block, sorts an array of strings; with a block,
  # sorts arbitrary items by the string the block returns (e.g. `thai_sort(users) { |u| u.name }`).
  # Returns a new array; input is left untouched.
  def self.thai_sort(items, &block)
    items.sort_by { |item| thai_sort_key(block ? block.call(item) : item) }
  end

  # Build the collation key for a string: an opaque binary String that compares (`<=>`, sort_by) and
  # stores (a DB sort column) in Royal-Institute order. Layout: primary letter weights, a 0x0000
  # separator, the secondary tone weights, then the reordered text as a final tiebreaker — each level
  # dominates the next. Non-Thai characters sort after Thai, ordered by codepoint (a Thai collator, so
  # rare astral-plane characters may share a bucket). Inherits sanitize_input hardening.
  def self.thai_sort_key(string)
    # NFC first so a below-vowel and a tone mark in non-canonical order (e.g. ร + ่ + ู vs ร + ู + ่,
    # which render identically) collate the same. Spaces are ignored (RI convention): "กรุง เทพ"
    # collates the same as "กรุงเทพ".
    cleaned = sanitize_input(string).unicode_normalize(:nfc).gsub(/[[:space:]]/, "")
    reordered = reorder_leading_vowels(cleaned)
    primary = []
    secondary = []
    i = 0
    while i < reordered.length
      ch = reordered[i]
      digraph = reordered[i + 1] ? ch + reordered[i + 1] : nil
      if digraph && (rank = PRIMARY_RANK[digraph]) # ฤๅ / ฦๅ read as one letter
        primary << rank
        secondary << 0
        i += 2
      elsif (rank = PRIMARY_RANK[ch])
        primary << rank
        secondary << 0
        i += 1
      elsif (tone = SECONDARY_RANK[ch])
        secondary[-1] = tone unless secondary.empty? # attach to the preceding letter's slot
        i += 1
      else
        primary << (0x2000 + (ch.ord & 0x1FFF)) # non-Thai: after all Thai letters
        secondary << 0
        i += 1
      end
    end

    separator = "\x00\x00".b
    key = String.new(encoding: Encoding::BINARY)
    key << primary.pack("n*") << separator << secondary.pack("n*") << separator << reordered.join.b
    key
  end

  # Swap each pre-posed vowel (เ แ โ ใ ไ) with the consonant that follows it, so the consonant is
  # compared first. Returns an array of characters. A leading vowel not followed by a consonant is
  # left in place.
  def self.reorder_leading_vowels(string)
    chars = string.chars
    out = []
    i = 0
    while i < chars.length
      current = chars[i]
      nxt = chars[i + 1]
      if COLLATION_LEADING_VOWELS.include?(current) && nxt && COLLATION_CONSONANTS.include?(nxt)
        out << nxt << current
        i += 2
      else
        out << current
        i += 1
      end
    end
    out
  end
  private_class_method :reorder_leading_vowels
end
