# tools/generate_char_map.rb
# One-off generator for the TIS 1566-2541 CHAR_TYPE table in lib/thapthim/tis_table.rb:
# prints the Thai-codepoint → character-class hash literal that file was seeded from.
# Kept for provenance; the committed table is the source of truth.
puts "{"
for char in (0xE01..0xE3A).to_a + (0xE3F..0xE5B).to_a
  case char
  when 0xE01..0xE23, 0xE27..0xE2E, 0xE25
    code = "CONS"
  when 0xE40..0xE44
    code = "LV"
  when 0xE30, 0xE32, 0xE33
    code = "FV1"
  when 0xE45
    code = "FV2"
  when 0xE24, 0xE26
    code = "FV3"
  when 0xE38
    code = "BV1"
  when 0xE39
    code = "BV2"
  when 0xE3A
    code = "BD"
  when 0xE48..0xE4B
    code = "TONE"
  when 0xE4C, 0xE4D
    code = "AD1"
  when 0xE47
    code = "AD2"
  when 0xE4E
    code = "AD3"
  when 0xE34
    code = "AV1"
  when 0xE31, 0xE36
    code = "AV2"
  when 0xE35, 0xE37
    code = "AV3"
  else
    code = "NON"
  end

  puts "  \"#{char.chr(Encoding::UTF_8)}\" => \"#{code}\", "
end
puts "}"