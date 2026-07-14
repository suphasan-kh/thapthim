# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# frozen_string_literal: true

module Thapthim
  # Deterministic numeral transforms: Buddhist/Common-era year conversion and Thai/Arabic digit
  # transliteration. Unlike segmentation and normalization these carry no model and no byte-level
  # subtlety, so they live in pure Ruby rather than the Rust core; a Python mirror is guaranteed
  # identical (integer arithmetic + a codepoint map over the BMP). The digit converters run input
  # through +sanitize_input+ so they inherit the same nil/encoding/NUL hardening as the rest of the
  # public API.

  # The Buddhist Era leads the Common Era by 543 years: BE = CE + 543 (e.g. CE 2025 = BE 2568).
  ERA_OFFSET = 543

  # Thai digits ๐–๙ (U+0E50–U+0E59) positionally aligned with Arabic 0–9 for a 1:1 +tr+ map.
  THAI_DIGITS   = "๐๑๒๓๔๕๖๗๘๙"
  ARABIC_DIGITS = "0123456789"

  # Buddhist Era year → Common Era year. Accepts an Integer or an integer-valued String
  # (e.g. 2568 or "2568"); raises ArgumentError/TypeError on non-integer input, matching Integer().
  # Purely arithmetic — no calendar/day handling (Thai and Gregorian year boundaries differ, but
  # the year-number offset is a fixed 543).
  def self.be2ce(year)
    Integer(year) - ERA_OFFSET
  end

  # Common Era year → Buddhist Era year. See +be2ce+ for input handling.
  def self.ce2be(year)
    Integer(year) + ERA_OFFSET
  end

  # Replace Thai digits ๐–๙ with Arabic 0–9 anywhere in the string; every other character is left
  # untouched. Returns a hardened UTF-8 String ("" for nil/empty input).
  def self.thai2arabic_digits(text)
    sanitize_input(text).tr(THAI_DIGITS, ARABIC_DIGITS)
  end

  # Replace Arabic digits 0–9 with Thai ๐–๙ anywhere in the string; every other character is left
  # untouched. Returns a hardened UTF-8 String ("" for nil/empty input).
  def self.arabic2thai_digits(text)
    sanitize_input(text).tr(ARABIC_DIGITS, THAI_DIGITS)
  end

  # --- number → Thai words --------------------------------------------------------
  #
  # The reader is +read_int+ (cardinal value reading). +num2text+ and +baht_text+ both delegate to
  # it and differ only in how they treat the fractional part: +num2text+ speaks a plain decimal
  # (จุด + digit-by-digit), +baht_text+ treats it as สตางค์. Keeping the currency logic out of the
  # core reader is deliberate — a number is not always money (see +num2text+ vs +baht_text+).

  # Digit words, indexed 0–9 (index 0 is ศูนย์, only ever spoken for the whole-number zero).
  DIGIT_WORDS = %w[ศูนย์ หนึ่ง สอง สาม สี่ ห้า หก เจ็ด แปด เก้า].freeze
  # Place words within a six-digit (sub-million) group, indexed by position: 0=units … 5=แสน.
  PLACE_WORDS = ["", "สิบ", "ร้อย", "พัน", "หมื่น", "แสน"].freeze

  # Read a non-negative Integer as Thai cardinal words. Millions are handled by recursion (Thai
  # groups by 10^6, so 10^12 reads ล้านล้าน). Encodes the three irregular readings: tens-1 is bare
  # สิบ (not หนึ่งสิบ), tens-2 is ยี่สิบ (not สองสิบ), and a units-1 that follows any higher place is
  # เอ็ด (not หนึ่ง) — the +!ret.empty?+ test carries that rule across the ล้าน boundary too.
  def self.read_int(number)
    return DIGIT_WORDS[0] if number.zero? # ศูนย์ only for a whole value of zero

    ret = +""
    if number >= 1_000_000
      ret << read_int(number / 1_000_000) << "ล้าน"
      number %= 1_000_000
    end

    divider = 100_000
    pos = 5
    while divider >= 1
      d = number / divider
      if d.positive?
        if divider == 10 && d == 1
          ret << "สิบ"
        elsif divider == 10 && d == 2
          ret << "ยี่สิบ"
        elsif divider == 1 && d == 1 && !ret.empty?
          ret << "เอ็ด"
        else
          ret << DIGIT_WORDS[d] << PLACE_WORDS[pos]
        end
      end
      number %= divider
      divider /= 10
      pos -= 1
    end
    ret
  end
  private_class_method :read_int

  # Parse an Integer, finite Float, or numeric String (Thai or Arabic digits, optional leading "-"
  # and fractional part) into [negative?, integer-part String, fractional-digit String]. Raises
  # ArgumentError on anything that is not a finite number.
  def self.parse_decimal(input)
    case input
    when Integer
      [input.negative?, input.abs.to_s, ""]
    when Float
      raise ArgumentError, "not a finite number: #{input.inspect}" unless input.finite?

      int_s, frac_s = input.abs.to_s.split(".") # Float#to_s is the shortest round-trip form
      [input.negative?, int_s, frac_s || ""]
    when String
      m = thai2arabic_digits(input).strip.match(/\A(-?)(\d+)(?:\.(\d+))?\z/)
      raise ArgumentError, "not a number: #{input.inspect}" unless m

      [m[1] == "-", m[2], m[3] || ""]
    else
      raise ArgumentError, "cannot read as a number: #{input.inspect}"
    end
  end
  private_class_method :parse_decimal

  # Read a number as plain Thai cardinal words — NOT currency. Integers read as a value; a fractional
  # part is spoken as จุด followed by each digit individually, exactly as given — every digit is kept,
  # including trailing zeros ("3.50" → "สามจุดห้าศูนย์", 3.0 → "สามจุดศูนย์"). Accepts Integer, finite
  # Float, or a numeric String (Thai or Arabic digits); a String preserves the digits verbatim,
  # sidestepping Float rounding. Negatives are prefixed ลบ.
  def self.num2text(number)
    negative, int_s, frac_s = parse_decimal(number)

    text = read_int(int_s.to_i)
    text += "จุด" + frac_s.chars.map { |c| DIGIT_WORDS[c.to_i] }.join unless frac_s.empty?
    negative ? "ลบ#{text}" : text
  end

  # Read the digits one at a time as Thai words — NOT as a value: 1234 → "หนึ่งสองสามสี่" (the value
  # reading is "หนึ่งพันสองร้อยสามสิบสี่"; see +num2text+). This is the reading for phone numbers, PINs,
  # years, and account numbers. Accepts an Integer or a string (Thai or Arabic digits); pass a string
  # to keep a leading zero ("081" → "ศูนย์แปดหนึ่ง"), which an Integer cannot carry. A "." is read as
  # จุด; every other character (spaces, dashes in a phone number) is ignored. "" when there are no digits.
  def self.read_digits(input)
    thai2arabic_digits(input).chars.filter_map do |ch|
      case ch
      when "0".."9" then DIGIT_WORDS[ch.to_i]
      when "."      then "จุด"
      end
    end.join
  end

  # Format a monetary amount as Thai baht text (บาท / สตางค์ / ถ้วน). The fractional part is rounded
  # to two decimal places and read as สตางค์; whole amounts end ถ้วน; a satang-only amount omits บาท.
  # Accepts Integer, finite Float, or numeric String — Strings round exactly (no Float error).
  # Negatives are prefixed ลบ. Currency lives here, not in +num2text+, because a number is not always
  # money and the decimal is read differently (สตางค์, not จุด).
  def self.baht_text(amount)
    negative, int_s, frac_s = parse_decimal(amount)

    # Round the fractional part to two digits (satang), carrying into baht if it rounds up to 100.
    satang = frac_s[0, 2].to_s.ljust(2, "0").to_i
    satang += 1 if frac_s.length > 2 && frac_s[2].to_i >= 5
    total_satang = (int_s.to_i * 100) + satang
    baht = total_satang / 100
    satang = total_satang % 100

    text =
      if satang.zero?
        "#{read_int(baht)}บาทถ้วน"
      elsif baht.zero?
        "#{read_int(satang)}สตางค์"
      else
        "#{read_int(baht)}บาท#{read_int(satang)}สตางค์"
      end
    negative ? "ลบ#{text}" : text
  end

  # --- Thai text → number (reverse of num2text) -----------------------------------

  # Digit/quantity words → value. เอ็ด and ยี่ are the positional variants of 1 and 2.
  UNIT_VALUES = {
    "ศูนย์" => 0, "หนึ่ง" => 1, "สอง" => 2, "สาม" => 3, "สี่" => 4,
    "ห้า" => 5, "หก" => 6, "เจ็ด" => 7, "แปด" => 8, "เก้า" => 9,
    "เอ็ด" => 1, "ยี่" => 2
  }.freeze
  # Place multipliers below one million.
  PLACE_VALUES = { "สิบ" => 10, "ร้อย" => 100, "พัน" => 1_000, "หมื่น" => 10_000, "แสน" => 100_000 }.freeze
  MILLION_WORD = "ล้าน"
  MINUS_WORD   = "ลบ"
  POINT_WORD   = "จุด"
  # Every recognized morpheme, longest first so tokenization is greedy. (No morpheme is a prefix of
  # another, so a single left-to-right longest match is unambiguous — Thai number words run together
  # with no spaces, e.g. "สองพันห้าร้อยหกสิบแปด".)
  NUMBER_MORPHEMES = (UNIT_VALUES.keys + PLACE_VALUES.keys + [MILLION_WORD, MINUS_WORD, POINT_WORD])
                     .sort_by { |w| -w.length }.freeze

  # Parse Thai number words back to a numeric value — the inverse of +num2text+. Understands the
  # place words (สิบ/ร้อย/พัน/หมื่น/แสน), the ล้าน grouping (including repeats, ล้านล้าน = 10^12), the
  # positional variants เอ็ด/ยี่, a leading ลบ, and a จุด decimal (digits after it read individually).
  # Returns an Integer for a whole number, a Float when a จุด is present (a value has no trailing
  # zeros, so "สามจุดห้าศูนย์" → 3.5). Whitespace is ignored; unrecognized input raises ArgumentError.
  def self.text2num(text)
    words = sanitize_input(text).gsub(/\s+/, "")
    raise ArgumentError, "not a number: #{text.inspect}" if words.empty?

    tokens = tokenize_number(words)

    negative = tokens.first == MINUS_WORD
    tokens = tokens.drop(1) if negative
    raise ArgumentError, "not a number: #{text.inspect}" if tokens.empty? || tokens.include?(MINUS_WORD)

    point = tokens.index(POINT_WORD)
    int_tokens  = point ? tokens[0...point] : tokens
    frac_tokens = point ? tokens[(point + 1)..] : []

    value = read_int_words(int_tokens)
    unless frac_tokens.empty?
      frac = frac_tokens.map { |w| UNIT_VALUES[w] || raise(ArgumentError, "not a digit after จุด: #{w}") }.join
      value = "#{value}.#{frac}".to_f
    end
    negative ? -value : value
  end

  # Greedy left-to-right longest match of the input into known morphemes (see NUMBER_MORPHEMES).
  def self.tokenize_number(words)
    tokens = []
    pos = 0
    while pos < words.length
      morpheme = NUMBER_MORPHEMES.find { |w| words[pos, w.length] == w }
      raise ArgumentError, "unrecognized number word at #{words[pos..].inspect}" if morpheme.nil?

      tokens << morpheme
      pos += morpheme.length
    end
    tokens
  end
  private_class_method :tokenize_number

  # Accumulate a token list into a non-negative Integer. +group+ builds the current sub-million value;
  # each ล้าน folds everything seen so far into +result+ and multiplies by 10^6 (so it composes for
  # ล้านล้าน). A bare place word (unit still 0, e.g. สิบ = 10) counts its digit as 1.
  def self.read_int_words(tokens)
    result = 0
    group = 0
    unit = 0
    tokens.each do |word|
      if UNIT_VALUES.key?(word)
        unit = UNIT_VALUES[word]
      elsif PLACE_VALUES.key?(word)
        group += (unit.zero? ? 1 : unit) * PLACE_VALUES[word]
        unit = 0
      elsif word == MILLION_WORD
        subtotal = result + group + unit
        subtotal = 1 if subtotal.zero? # a leading/bare ล้าน means 1,000,000
        result = subtotal * 1_000_000
        group = 0
        unit = 0
      else
        raise ArgumentError, "unexpected word: #{word}"
      end
    end
    result + group + unit
  end
  private_class_method :read_int_words
end
