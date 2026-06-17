# SPDX-FileCopyrightText: 2016-2026 PyThaiNLP Project
# SPDX-FileCopyrightText: 2026 Thapthim Project Contributor suphasan-kh
# SPDX-FileType: SOURCE
# SPDX-License-Identifier: Apache-2.0

# frozen_string_literal: true

require_relative "thapthim/version"

module Thapthim
  class Error < StandardError; end
  class << self
    include Thapthim
  end
end

order = ["tis_table", "valid_tis", "normalize_tis", "normalize_std", "segment_tcc_legacy", "segment_tcc"]

order.each { |file| require_relative "thapthim/#{file}"}