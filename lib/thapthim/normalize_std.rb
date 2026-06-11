module Thapthim
  # Thai text normalization function in Ruby, inspired by pythainlp.util.normalize

  # Arrays of each char type
  ZW_CHARS = ["\u200B", "\u200C", "\u200D"] # ZWSP, ZWNJ, ZWJ 
  CONSONANTS = "กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรฤลฦวศษสหฬอฮ".split("")
  ABOVE_VOWELS = ["\u0E31", "\u0E34", "\u0E35", "\u0E36", "\u0E37", "\u0E4D", "\u0E47"] # ั, ิ, ี, ึ, ื, ํ, ็
  BELOW_VOWELS = ["\u0E38", "\u0E39"] # ุ, ู
  LEAD_VOWELS = ["\u0E40", "\u0E41", "\u0E42", "\u0E43", "\u0E44"] # เ, แ, โ, ใ, ไ
  FOLLOW_VOWELS = ["\u0E30", "\u0E32", "\u0E33"] # ะ, า, ำ
  VOWELS = [ABOVE_VOWELS, BELOW_VOWELS, LEAD_VOWELS, FOLLOW_VOWELS, "\u0E24", "\u0E26"].flatten # ฤ, ฦ
  TONES = ["\u0E48", "\u0E49", "\u0E4A", "\u0E4B"] # ่, ้, ๊, ๋
  NON_BASE = [ABOVE_VOWELS, BELOW_VOWELS, TONES, "\u0E3A", "\n0E4C", "\n0E4E"].flatten #  ฺ, ์, ๎ 

  private_constant :ZW_CHARS, :CONSONANTS, :ABOVE_VOWELS, :BELOW_VOWELS, :LEAD_VOWELS, :FOLLOW_VOWELS, :VOWELS, :TONES, :NON_BASE

  def self.normalize_std(input)
    return input if input.nil? || input.empty?

    # Input has to be a string, if not, convert it to string
    text = String.new(input)
    normalized = String.new(capacity: text.bytesize)
    buffer = ""

    text.each_char do |char|
      if ZW_CHARS.include?(char)
        # Remove zero-width characters
        next
      elsif char == " " && normalized[-1] == " "
        # Compress consecutive spaces into a single space
        next
      elsif NON_BASE.include?(char) && normalized[-1] == " " && CONSONANTS.include?(normalized[-2]) && !VOWELS.include?(normalized[-3])
        # Remove spaces before tone marks and non-base characters that follow a consonant except precede by a vowel
        normalized[-1] = char
      elsif char == "า" && normalized[-1] == "ํ"
        # Combine "า" + "ํ" into "ำ"
        normalized[-1] = "ำ"
      elsif char == "เ" && normalized[-1] == "เ"
        # Combine "เ" + "เ" into "แ"
        normalized[-1] = "แ"
      elsif char == "ๅ" && !(["\u0E24\u0E26"].include?(char))
        # Change "ๅ" to "า" if the previous character is not ฤ,ฦ
        normalized << "า"
      else
        normalized << char
      end
    end
    return normalized
  end
end