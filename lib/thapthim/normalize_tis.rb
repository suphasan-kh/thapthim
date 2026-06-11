module Thapthim
  # Normalize Thai input string according to TIS 1566-2541 standard
  def self.normalize_tis(input, strict=false)
    return input if input.nil? || input.empty?

    # Input has to be a string, if not, convert it to string
    text = " " + String.new(input)
    normalized = String.new(capacity: text.bytesize)

    for i in 1...text.length
      char = text[i]
      prev_char = text[i-1]
      char_type = CHAR_TYPE[char]
      prev_char_type = CHAR_TYPE[prev_char]
      rule = RULES[prev_char_type][char_type]
      case rule
      when 0
        normalized << char
      when 1
        next
      when 2
        if strict
          next
        else
          normalized << char
        end
      end
    end
    return normalized
  end
end