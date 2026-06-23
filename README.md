# Thapthim

TODO: Delete this and the text below, and describe your gem

Welcome to your new gem! In this directory, you'll find the files you need to be able to package up your Ruby library into a gem. Put your Ruby code in the file `lib/thapthim`. To experiment with that code, run `bin/console` for an interactive prompt.

## Installation

TODO: Replace `UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG` with your gem name right after releasing it to RubyGems.org. Please do not do it earlier due to security reasons. Alternatively, replace this section with instructions to install your gem from git if you don't plan to release to RubyGems.org.

Install the gem and add to the application's Gemfile by executing:

```bash
bundle add UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG
```

If bundler is not being used to manage dependencies, install the gem by executing:

```bash
gem install UPDATE_WITH_YOUR_GEM_NAME_IMMEDIATELY_AFTER_RELEASE_TO_RUBYGEMS_ORG
```

## Usage

TODO: Write usage instructions here

## Development

After checking out the repo, run `bin/setup` to install dependencies. Then, run `rake spec` to run the tests. You can also run `bin/console` for an interactive prompt that will allow you to experiment.

To install this gem onto your local machine, run `bundle exec rake install`. To release a new version, update the version number in `version.rb`, and then run `bundle exec rake release`, which will create a git tag for the version, push git commits and the created tag, and push the `.gem` file to [rubygems.org](https://rubygems.org).

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/[USERNAME]/thapthim. This project is intended to be a safe, welcoming space for collaboration, and contributors are expected to adhere to the [code of conduct](https://github.com/[USERNAME]/thapthim/blob/main/CODE_OF_CONDUCT.md).

## License

**Source code:** [MIT License](https://opensource.org/licenses/MIT) (see [LICENSE.txt](LICENSE.txt)).

**The gem as a whole is for non-commercial / research / open-source use only.** This is *not* a
choice — it follows from the bundled model assets, which are derived from non-commercial corpora:

- The dictionary includes vocabulary from the **BEST** corpus — **CC BY-NC-SA 3.0** (NonCommercial,
  ShareAlike).
- The n-gram language model is trained on the **LST20** corpus — NECTEC's agreement permits
  non-commercial/research/open-source use **only**, and **requires citing** the LST20 report
  (Boonkwan et al., 2020, *The Annotation Guideline of LST20 Corpus*). Commercial use requires a
  separate license from NECTEC.

So while the code is MIT, you **may not use the gem commercially** without resolving the corpus
licenses yourself. It also bundles PyThaiNLP's TCC/normalization components (Apache-2.0). Full
attribution and per-source terms are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Code of Conduct

Everyone interacting in the Thapthim project's codebases, issue trackers, chat rooms and mailing lists is expected to follow the [code of conduct](https://github.com/suphasan-kh/thapthim/blob/main/CODE_OF_CONDUCT.md).
