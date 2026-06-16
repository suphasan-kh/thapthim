# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# Input validation from TIS 1566-2541

module Thapthim
    def self.tis_valid(input, strict=false)
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