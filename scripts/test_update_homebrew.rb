require "digest"
require "fileutils"
require "open3"
require "rbconfig"
require "tmpdir"

updater = File.expand_path("update_homebrew.rb", __dir__)
targets = %w[aarch64-apple-darwin x86_64-apple-darwin]
Dir.mktmpdir do |dir|
  formula = File.join(dir, "cockup.rb")
  original = "class Cockup < Formula\n" + targets.map do |target|
    "  url \"https://github.com/huaium/cockup/releases/download/v0.2.1/cockup-v0.2.1-#{target}.tar.gz\"\n  sha256 \"#{'a' * 64}\"\n"
  end.join + "  # Retain customizations\nend\n"
  File.write(formula, original)
  targets.each do |target|
    archive = "cockup-v0.2.2-#{target}.tar.gz"
    File.write(File.join(dir, archive), target)
    File.write(File.join(dir, "#{archive}.sha256"), "#{Digest::SHA256.hexdigest(target)}  #{archive}\n")
  end
  run = ->(tag) { Open3.capture3(RbConfig.ruby, updater, tag, formula, dir) }
  # A bad second asset must not partially update the first architecture.
  checksum = File.join(dir, "cockup-v0.2.2-x86_64-apple-darwin.tar.gz.sha256")
  correct = File.read(checksum)
  File.write(checksum, "invalid")
  _, _, status = run.call("v0.2.2")
  raise "Accepted invalid checksum or changed formula" if status.success? || File.read(formula) != original
  File.write(checksum, correct)
  _, stderr, status = run.call("v0.2.2")
  raise stderr unless status.success?
  updated = File.read(formula)
  raise "Incomplete update" unless updated.scan("download/v0.2.2/").length == 2 && updated.include?("Retain customizations")
  targets.each { |target| raise "Wrong checksum" unless updated.include?(Digest::SHA256.hexdigest(target)) }
  _, stderr, status = run.call("v0.2.2")
  raise "Not idempotent: #{stderr}" unless status.success? && File.read(formula) == updated
  ["v0.2.1", "invalid"].each do |tag|
    _, _, status = run.call(tag)
    raise "Accepted downgrade/invalid tag" if status.success? || File.read(formula) != updated
  end
end
puts "Homebrew updater checks passed"
