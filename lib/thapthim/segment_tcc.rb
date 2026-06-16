# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0
=begin 
The implementation of tokenizer according to Thai Character Clusters (TCCs) rules proposed by `Theeramunkong et al. 2000. \
    <https://doi.org/10.1145/355214.355225>`_

Credits:
    * TCC: Jakkrit TeCho
    * Grammar: Wittawat Jitkrittum (`link to the source file \
      <https://github.com/wittawatj/jtcc/blob/master/TCC.g>`_)
    * Python code: Korakot Chaovavanich
=end

module Thapthim
    RE_TCC = "
        c[ั]([่-๋]c)?
        c[ั]([่-๋]c)?k
        เc็ck
        เcctาะk
        เccีtยะk
        เccีtย(?=[เ-ไก-ฮ]|$)k
        เc[ิีุู]tย(?=[เ-ไก-ฮ]|$)k
        เcc็ck
        เcิc์ck
        เcิtck
        เcีtยะ?k
        เcืtอะk
        เcื
        เctา?ะ?k
        c[ึื]tck
        c[ะ-ู]tk
        c[ิุู]์
        cรรc์
        c็
        ct[ะาำ]?k
        แc็ck
        แcc์k
        แctะk
        แcc็ck
        แccc์k
        โctะk
        [เ-ไ]ctk
        ก็
        อึ
        หึ
    ".gsub("k", "(XX?(ุ|ู|ิ)?[์])?")
    .gsub("c", "[ก-ฮ]")
    .gsub("X", "[ก-ฮ]")
    .gsub("t", "[่-๋]?")
    .strip.split(/\s+/).join("|")
    def self.tcc_segment(input)
        return [] if input.nil? || input.empty?
        String.new(input).gsub(/#{RE_TCC}|./).to_a
    end

    def self.tcc_positions(tcc_tokens)
        positions = [0]
        current_len = 0
        
        tcc_tokens.each do |chunk|
        current_len += chunk.length
        positions << current_len
        end
        
        positions
    end
end