module Thapthim
  def self.valid_tis(input, strict=false)
      return true if input.nil? || input.empty?
      
      text = String.new(input)
      prev_char_type = :NON
      
      text.each_char do |char|
          char_type = CHAR_TYPE[char]
          rule = RULES[prev_char_type][char_type]
          case rule
          when :A
              prev_char_type = char_type
          when :R
              return false
          when :S
              if strict
                return false
              else
                prev_char_type = char_type
              end
          end
      end
      return true
  end
end