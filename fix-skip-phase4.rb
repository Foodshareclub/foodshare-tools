#!/usr/bin/env ruby
# Phase 4: Wrap remaining unwrapped files that shouldn't be visible to Android

APP_ROOT = "/Users/organic/dev/work/foodshare/foodshare-app"
SRC_ROOT = "#{APP_ROOT}/Sources/FoodShare"

def wrap_file(path)
  return unless File.exist?(path)
  content = File.read(path)
  lines = content.lines

  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }

  return if first_code && last_code &&
            lines[first_code]&.strip == "#if !SKIP" &&
            lines[last_code]&.strip == "#endif"

  header_end = 0
  lines.each_with_index do |line, i|
    stripped = line.strip
    if stripped.start_with?("//") || stripped.empty?
      header_end = i + 1
    else
      break
    end
  end

  header = lines[0...header_end].join
  body = lines[header_end..].join

  new_content = "#{header}\n#if !SKIP\n#{body}\n#endif\n"
  File.write(path, new_content)
  puts "  Wrapped: #{path.sub("#{SRC_ROOT}/", "")}"
end

# Wrap all theme-related files (NOT the token files)
Dir.glob("#{SRC_ROOT}/Core/Design/Theming/**/*.swift").each do |f|
  wrap_file(f)
end

# Now find ALL remaining unwrapped files (except tokens and Skip/)
whitelist_patterns = [
  "FoodShareApp.swift",
  "Skip/",
  "Core/Design/Tokens/",
]

Dir.glob("#{SRC_ROOT}/**/*.swift").each do |path|
  rel = path.sub("#{SRC_ROOT}/", "")
  next if whitelist_patterns.any? { |p| rel.start_with?(p) || rel == p }

  content = File.read(path)
  lines = content.lines
  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }

  is_wrapped = first_code && last_code &&
               lines[first_code]&.strip == "#if !SKIP" &&
               lines[last_code]&.strip == "#endif"

  # Also check if file starts with #if SKIP (has proper Skip support)
  has_skip_support = first_code && lines[first_code]&.strip == "#if SKIP"

  unless is_wrapped || has_skip_support
    wrap_file(path)
  end
end

# Summary
unwrapped = Dir.glob("#{SRC_ROOT}/**/*.swift").count do |path|
  content = File.read(path)
  lines = content.lines
  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }
  !(first_code && last_code &&
    (lines[first_code]&.strip == "#if !SKIP" || lines[first_code]&.strip == "#if SKIP") &&
    lines[last_code]&.strip == "#endif")
end
puts "\nRemaining unwrapped files: #{unwrapped}"
