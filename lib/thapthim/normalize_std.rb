# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

module Thapthim
  # Thai text normalization in Ruby, modified from PyThaiNLP
  CONSONANTS = "\u0E01-\u0E23\u0E25\u0E27-\u0E2E"
  ABOVE_VOWELS = "\u0E31\u0E34-\u0E37\u0E4D\u0E47" # ั, ิ, ี, ึ, ื, ํ, ็
  BELOW_VOWELS = "\u0E38\u0E39" # ุ, ู
  LEAD_VOWELS = "\u0E40-\u0E44" # เ, แ, โ, ใ, ไ
  FOLLOW_VOWELS = "\u0E30\u0E32\u0E33" # ะ, า, ำ
  VOWELS = "#{ABOVE_VOWELS}#{BELOW_VOWELS}#{LEAD_VOWELS}#{FOLLOW_VOWELS}\u0E24\u0E26" # ฤ, ฦ
  TONES = "\u0E48-\u0E4B" # ่, ้, ๊, ๋

  ZW_CHARS = "\u200B\u200C"
  DANGLING_CHARS = "#{ABOVE_VOWELS}#{BELOW_VOWELS}#{TONES}\u0E3A\u0E4C\u0E4D\u0E4E"
  NOREPEAT_CHARS = "#{FOLLOW_VOWELS}#{LEAD_VOWELS}#{ABOVE_VOWELS}#{BELOW_VOWELS}\u0E3A\u0E4C\u0E4D\u0E4E" #

  REORDER_PAIRS = [
    [/\u0e40\u0e40/, "\u0e41"], # Combine เ + เ = แ
    [/([#{TONES}\u0E4C]+)([#{ABOVE_VOWELS}#{BELOW_VOWELS}]+)/, "\\2\\1"], # Swap tones and  ์ with above and below vowels
    [/\u0E4D([#{TONES}]*)\u0E32/, "\\1\u0E33"], # Combine  ํ + tone + า = tone + ำ
    [/([#{FOLLOW_VOWELS}]+)([#{TONES}]+)/, "\\2\\1"], # Swap follow vowels with tones
    [/([^\u0E24\u0E26])\u0E45/, "\\1\u0E32"] # Change ๅ to า if not precede by ฤ, ฦ
  ].freeze  

  private_constant :CONSONANTS, :ABOVE_VOWELS, :BELOW_VOWELS, :LEAD_VOWELS, :FOLLOW_VOWELS, :VOWELS, :TONES, :DANGLING_CHARS, :NOREPEAT_CHARS, :REORDER_PAIRS

  def self.remove_zw(text)
    text.gsub(/[#{ZW_CHARS}]/,"")
  end

  def self.remove_dup_spaces(text)
    text = text.gsub(/[ \n]*\n[ \n]*/, "\n")
    text.gsub!(/ +/, " ")
    text ? text.strip : ""
  end

  def self.remove_spaces_before_marks(text)
    text.gsub(/([#{CONSONANTS}])(?<![#{VOWELS}][#{CONSONANTS}]) ([#{DANGLING_CHARS}])/, "\\1\\2")
  end

  def self.reorder_vowels(text)
    REORDER_PAIRS.each do |pattern, replace|
      text = text.gsub(pattern, replace)
    end
    text
  end

  def self.remove_repeat_vowels(text)
    text = reorder_vowels(text)

    NOREPEAT_CHARS.each_char do |ch|
      pattern = /(#{Regexp.escape(ch)}[ ]*)+#{Regexp.escape(ch)}/
      text = text.gsub(pattern, ch)
    end

    text.gsub(/[#{TONES}]+/) { |match| match[-1] }
  end

  def self.remove_dangling(text)
    text = text.sub(/^[#{DANGLING_CHARS}]+/, "")
    text.gsub(/ +[#{DANGLING_CHARS}]+/, " ")
  end
    
  def self.std_normalize(input)
    return input if input.nil? || input.empty?
    text = String.new(input)
    text = remove_zw(text)
    text = remove_dup_spaces(text)
    text = remove_spaces_before_marks(text)
    text = remove_repeat_vowels(text)
    text = remove_dangling(text)

    text
  end
end