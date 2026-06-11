# frozen_string_literal: true

require_relative "thapthim/version"

module Thapthim
  class Error < StandardError; end
  # Your code goes here...
end

order = ["tis_table", "valid_tis", "normalize_tis", "normalize_std"]

order.each { |file| require_relative "thapthim/#{file}"}