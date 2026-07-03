# frozen_string_literal: true

source "https://rubygems.org"

# Specify your gem's dependencies in thapthim.gemspec
gemspec

gem "irb"
gem "rake", "~> 13.0"

gem "minitest", "~> 5.0"

gem "standard", "~> 1.3"

# Transitive dep of standard/rubocop; parallel 2.x requires Ruby >= 3.3, which breaks
# `bundle install` on Ruby 3.2 (the oldest version this gem supports — see CI).
gem "parallel", "~> 1.26"
