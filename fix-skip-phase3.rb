#!/usr/bin/env ruby
# Phase 3: Unwrap over-wrapped files and take a smarter approach
# Strategy: Wrap EVERYTHING in #if !SKIP except what's needed for Android

APP_ROOT = "/Users/organic/dev/work/foodshare/foodshare-app"
SRC_ROOT = "#{APP_ROOT}/Sources/FoodShare"

$changes = { unwrapped: [], wrapped: [] }

# ============================================================
# Helper: Remove #if !SKIP / #endif file-level wrapping
# ============================================================

def unwrap_file(path)
  return unless File.exist?(path)
  content = File.read(path)
  lines = content.lines

  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }

  return unless first_code && last_code &&
                lines[first_code]&.strip == "#if !SKIP" &&
                lines[last_code]&.strip == "#endif"

  # Remove the #if !SKIP line and last #endif line
  new_lines = lines.dup
  new_lines.delete_at(last_code)
  new_lines.delete_at(first_code)

  File.write(path, new_lines.join)
  $changes[:unwrapped] << path.sub("#{SRC_ROOT}/", "")
end

# ============================================================
# Helper: Wrap file if not already wrapped
# ============================================================

def wrap_file(path)
  return unless File.exist?(path)
  content = File.read(path)
  lines = content.lines

  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }

  return if first_code && last_code &&
            lines[first_code]&.strip == "#if !SKIP" &&
            lines[last_code]&.strip == "#endif"

  # Remove existing partial #if !SKIP / #endif pairs for imports
  content = content.gsub(/^#if !SKIP\nimport CoreData\n#endif\n/, "import CoreData\n")

  header_lines = content.lines
  header_end = 0
  header_lines.each_with_index do |line, i|
    stripped = line.strip
    if stripped.start_with?("//") || stripped.empty?
      header_end = i + 1
    else
      break
    end
  end

  header = header_lines[0...header_end].join
  body = header_lines[header_end..].join

  new_content = "#{header}\n#if !SKIP\n#{body}\n#endif\n"
  File.write(path, new_content)
  $changes[:wrapped] << path.sub("#{SRC_ROOT}/", "")
end

# ============================================================
# Strategy: Wrap ALL files EXCEPT a whitelist
# The whitelist contains only what's needed for Android to compile
# ============================================================

# Files that MUST NOT be wrapped (needed for Android build)
whitelist = [
  # Entry point
  "FoodShareApp.swift",

  # Skip compatibility layer
  "Skip/skip.yml",
  "Skip/UIKitCompat.swift",
  "Skip/CoreLocationCompat.swift",

  # Design tokens (have #if SKIP / #else properly)
  "Core/Design/Tokens/LiquidGlassColors.swift",
  "Core/Design/Tokens/LiquidGlassSpacing.swift",
  "Core/Design/Tokens/LiquidGlassTypography.swift",
  "Core/Design/Tokens/LiquidGlassCornerRadius.swift",

  # Theme files (have proper Skip support)
  "Core/Design/Theming/BrandTheme.swift",
  "Core/Design/Theming/CoralTheme.swift",
  "Core/Design/Theming/ForestTheme.swift",
  "Core/Design/Theming/MidnightTheme.swift",
  "Core/Design/Theming/MonochromeTheme.swift",
  "Core/Design/Theming/NatureTheme.swift",
  "Core/Design/Theming/OceanTheme.swift",
  "Core/Design/Theming/SunsetTheme.swift",
  "Core/Design/Theming/ThemedColors.swift",
  "Core/Design/Theming/ThemeManager.swift",
  "Core/Design/Theming/ThemeEnvironment.swift",
]

# First: unwrap ALL currently-wrapped files
Dir.glob("#{SRC_ROOT}/**/*.swift").each do |path|
  unwrap_file(path)
end

# Then: wrap everything EXCEPT whitelist
whitelist_full = whitelist.map { |f| "#{SRC_ROOT}/#{f}" }

Dir.glob("#{SRC_ROOT}/**/*.swift").each do |path|
  next if whitelist_full.include?(path)
  next if path.include?("/Skip/") # Don't wrap anything in Skip/ directory
  wrap_file(path)
end

# ============================================================
# Summary
# ============================================================

puts "=" * 60
puts "Phase 3 Fix Summary"
puts "=" * 60
puts ""
puts "Files unwrapped: #{$changes[:unwrapped].size}"
puts "Files wrapped in #if !SKIP: #{$changes[:wrapped].size}"
puts ""

# Count files NOT wrapped
not_wrapped = Dir.glob("#{SRC_ROOT}/**/*.swift").count do |path|
  content = File.read(path)
  lines = content.lines
  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }
  !(first_code && last_code &&
    lines[first_code]&.strip == "#if !SKIP" &&
    lines[last_code]&.strip == "#endif")
end
puts "Files NOT wrapped (Android-visible): #{not_wrapped}"
