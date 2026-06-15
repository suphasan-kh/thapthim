# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

module Thapthim
  # Normalize Thai input string according to TIS 1566-2541 standard
  def self.normalize_tis(input, strict=false)
    return input if input.nil? || input.empty?

    # Input has to be a string, if not, convert it to string
    cleaned = self.normalize_std(input)
    text = String.new(cleaned)
    normalized = String.new(capacity: text.bytesize)
    prev_char_type = :NON

    text.each_char do |char|
      char_type = CHAR_TYPE[char]
      rule = RULES[prev_char_type][char_type]
      case rule
      when :A
        normalized << char
        prev_char_type = char_type
      when :R
        next
      when :S
        if strict
          next
        else
          normalized << char
          prev_char_type = char_type
        end
      end
    end
    return normalized
  end
end