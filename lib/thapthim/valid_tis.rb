require_relative "tis_table"

module Thapthim
  def self.valid_tis(input, strict=false)
      return true if input.nil? || input.empty?
      
      text = " " + String.new(input)
      
      for i in 1...text.length
          char = text[i]
          prev_char = text[i-1]
          char_type = CHAR_TYPE[char]
          prev_char_type = CHAR_TYPE[prev_char]
          rule = RULES[prev_char_type][char_type]
          case rule
          when 0
              next
          when 1
              return false
          when 2
              return false if strict
          end
      end
      return true
  end
end