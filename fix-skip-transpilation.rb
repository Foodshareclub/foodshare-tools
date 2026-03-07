#!/usr/bin/env ruby
# Fix Skip transpilation errors for FoodShare Android build
# Wraps iOS-only files in #if !SKIP guards and fixes common patterns

require 'fileutils'

APP_ROOT = "/Users/organic/dev/work/foodshare/foodshare-app"
SRC_ROOT = "#{APP_ROOT}/Sources/FoodShare"

# Track changes
$changes = { wrapped: [], material_fixed: [], int_fixed: [], liquid_glass_fixed: [] }

# ============================================================
# PHASE 1: Wrap entire files in #if !SKIP
# These files use iOS-only APIs (supabase-swift, CoreData, etc.)
# ============================================================

def wrap_file_in_skip_guard(path)
  return unless File.exist?(path)
  content = File.read(path)

  # Skip if already has file-level guard
  lines = content.lines
  # Check if the first non-comment, non-blank line is #if !SKIP
  code_start = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  return if code_start && lines[code_start]&.strip == "#if !SKIP"

  # Find where the header comments end
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
  $changes[:wrapped] << path.sub("#{SRC_ROOT}/", "")
end

# Files to wrap entirely in #if !SKIP
files_to_wrap = []

# All Supabase repositories (use supabase-swift)
Dir.glob("#{SRC_ROOT}/**/Supabase*Repository.swift").each { |f| files_to_wrap << f }

# Database layer (use supabase-swift)
%w[DatabaseService.swift RealtimeService.swift StorageService.swift
   SupabaseDataFetching.swift SupabaseManager.swift].each do |f|
  files_to_wrap << "#{SRC_ROOT}/Core/Database/#{f}"
end

# All API services (use APIClient which uses supabase-swift)
Dir.glob("#{SRC_ROOT}/Core/Services/*APIService.swift").each { |f| files_to_wrap << f }
Dir.glob("#{SRC_ROOT}/Core/Services/*Service.swift").each { |f| files_to_wrap << f }

# Core networking (APIClient uses supabase-swift)
Dir.glob("#{SRC_ROOT}/Core/Networking/*Service.swift").each { |f| files_to_wrap << f }
%w[APIClient.swift EdgeFunctionResponse.swift EdgeFunctionError.swift].each do |f|
  path = "#{SRC_ROOT}/Core/Networking/#{f}"
  files_to_wrap << path if File.exist?(path)
end

# Security services (iOS-only: DeviceCheck, AppAttest, Keychain)
Dir.glob("#{SRC_ROOT}/Core/Security/*Service.swift").each { |f| files_to_wrap << f }

# Notifications (iOS-only: UserNotifications, APNS)
Dir.glob("#{SRC_ROOT}/Core/Notifications/*Service.swift").each { |f| files_to_wrap << f }

# Storage/Cache services
Dir.glob("#{SRC_ROOT}/Core/Storage/*Service.swift").each { |f| files_to_wrap << f }
Dir.glob("#{SRC_ROOT}/Core/Cache/*Service.swift").each { |f| files_to_wrap << f }

# Feature-level services
Dir.glob("#{SRC_ROOT}/Features/**/Services/*.swift").each { |f| files_to_wrap << f }
Dir.glob("#{SRC_ROOT}/Features/**/Domain/Services/*.swift").each { |f| files_to_wrap << f }

# Specific iOS-only files
ios_only_files = %w[
  Core/Services/AdminAuthorizationService.swift
  Core/Services/AuthenticationService.swift
  Core/Services/CacheWarmingService.swift
  Core/Services/InvitationService.swift
  Core/Services/LegalDocumentService.swift
  Core/Services/ResendService.swift
  Core/Services/StoreKitService.swift
  Core/Services/UpstashService.swift
  Core/Services/ErrorRecoveryService.swift
  Core/Services/ImagePrefetchService.swift
  Core/Services/ShareService.swift
  Core/Error/ErrorReportingService.swift
  Core/Compliance/GDPRExportService.swift
  Core/Location/IPGeolocationService.swift
  Core/Location/MockLocationService.swift
  Core/Design/Utilities/ShaderLibraryExtensions.swift
  Features/Feedback/Data/SupabaseFeedbackRepository.swift
  Features/Reports/Data/SupabaseReportRepository.swift
  Features/Settings/Domain/Services/MFAService.swift
  Features/Settings/Domain/Services/SettingsBackupService.swift
  Features/Forum/Data/Repositories/SupabaseForumCommentRepository.swift
  Features/Forum/Data/Repositories/SupabaseForumEngagementRepository.swift
  Features/Forum/Data/Repositories/SupabaseForumPollRepository.swift
  Features/Forum/Data/Repositories/SupabaseForumReputationRepository.swift
  Features/Forum/Domain/Services/ForumRealtimeService.swift
  Features/Messaging/Domain/Services/ChatPresenceService.swift
  Features/Map/Services/MapPreferencesService.swift
  Features/Notifications/Data/Repositories/SupabaseNotificationPreferencesRepository.swift
  Core/Notifications/EmailService.swift
  Core/Notifications/PushNotificationService.swift
]

ios_only_files.each { |f| files_to_wrap << "#{SRC_ROOT}/#{f}" }

# Deduplicate and wrap
files_to_wrap.uniq.sort.each { |f| wrap_file_in_skip_guard(f) }

# ============================================================
# PHASE 2: Fix .ultraThinMaterial references
# Replace with Skip-compatible alternatives
# ============================================================

def fix_ultra_thin_material(path)
  return unless File.exist?(path)
  content = File.read(path)
  return unless content.include?("ultraThinMaterial")

  # Already has guard for ultraThinMaterial specifically
  return if content.include?("#if !SKIP") && content.include?("ultraThinMaterial") &&
            content.lines.any? { |l| l.include?("#if !SKIP") && content.index(l) &&
              content[content.index(l)..content.index(l)+200].include?("ultraThinMaterial") }

  original = content.dup

  # Pattern: .fill(.ultraThinMaterial) → platform guard
  content.gsub!(/^(\s*)\.fill\(\.ultraThinMaterial\)/) do
    indent = $1
    "#{indent}#if !SKIP\n#{indent}.fill(.ultraThinMaterial)\n#{indent}#else\n#{indent}.fill(Color.DesignSystem.glassSurface.opacity(0.15))\n#{indent}#endif"
  end

  # Pattern: .background(.ultraThinMaterial) → platform guard
  content.gsub!(/^(\s*)\.background\(\.ultraThinMaterial\)/) do
    indent = $1
    "#{indent}#if !SKIP\n#{indent}.background(.ultraThinMaterial)\n#{indent}#else\n#{indent}.background(Color.DesignSystem.glassSurface.opacity(0.15))\n#{indent}#endif"
  end

  # Pattern: .background(.ultraThinMaterial, in: ...) → platform guard
  content.gsub!(/^(\s*)\.background\(\.ultraThinMaterial,\s*in:\s*([^)]+)\)/) do
    indent = $1
    shape = $2
    "#{indent}#if !SKIP\n#{indent}.background(.ultraThinMaterial, in: #{shape})\n#{indent}#else\n#{indent}.background(Color.DesignSystem.glassSurface.opacity(0.15))\n#{indent}#endif"
  end

  # Pattern: .toolbarBackground(.ultraThinMaterial, ...) → platform guard
  content.gsub!(/^(\s*)\.toolbarBackground\(\.ultraThinMaterial,\s*for:\s*([^)]+)\)/) do
    indent = $1
    bar = $2
    "#{indent}#if !SKIP\n#{indent}.toolbarBackground(.ultraThinMaterial, for: #{bar})\n#{indent}#endif"
  end

  # Note: remaining .ultraThinMaterial in complex expressions left for manual fix

  if content != original
    File.write(path, content)
    $changes[:material_fixed] << path.sub("#{SRC_ROOT}/", "")
  end
end

# Fix ultraThinMaterial in all UI files
Dir.glob("#{SRC_ROOT}/**/*.swift").each { |f| fix_ultra_thin_material(f) }

# ============================================================
# PHASE 3: Fix LiquidGlass → DesignSystem references
# Font.LiquidGlass.xxx → Font.DesignSystem.xxx
# Color.LiquidGlass.xxx → Color.DesignSystem.xxx
# ============================================================

Dir.glob("#{SRC_ROOT}/**/*.swift").each do |path|
  content = File.read(path)
  next unless content.include?("LiquidGlass")

  original = content.dup

  # Font.LiquidGlass → Font.DesignSystem (LiquidGlass is iOS-only alias)
  content.gsub!(/Font\.LiquidGlass\./, "Font.DesignSystem.")

  # Color.LiquidGlass → Color.DesignSystem (if used anywhere)
  content.gsub!(/Color\.LiquidGlass\./, "Color.DesignSystem.")

  if content != original
    File.write(path, content)
    $changes[:liquid_glass_fixed] << path.sub("#{SRC_ROOT}/", "")
  end
end

# ============================================================
# PHASE 4: Fix Int → Double literal issues in UI files
# CGFloat/Double contexts need .0 suffix on integer literals
# ============================================================

def fix_int_literals(path)
  return unless File.exist?(path)
  content = File.read(path)
  original = content.dup

  # CGSize(width: 0, height: 0) → CGSize(width: 0.0, height: 0.0)
  content.gsub!(/CGSize\(width:\s*(\d+),\s*height:\s*(\d+)\)/) do
    w, h = $1, $2
    w_val = w.include?(".") ? w : "#{w}.0"
    h_val = h.include?(".") ? h : "#{h}.0"
    "CGSize(width: #{w_val}, height: #{h_val})"
  end

  # CGPoint(x: 0, y: 0) → CGPoint(x: 0.0, y: 0.0)
  content.gsub!(/CGPoint\(x:\s*(\d+),\s*y:\s*(\d+)\)/) do
    x, y = $1, $2
    x_val = x.include?(".") ? x : "#{x}.0"
    y_val = y.include?(".") ? y : "#{y}.0"
    "CGPoint(x: #{x_val}, y: #{y_val})"
  end

  # Common patterns: max(0, ...) → max(0.0, ...) when used with CGFloat
  content.gsub!(/\bmax\(0,\s/, "max(0.0, ")
  content.gsub!(/\bmin\(0,\s/, "min(0.0, ")
  content.gsub!(/\bmax\(1,\s/, "max(1.0, ")
  content.gsub!(/\bmin\(1,\s/, "min(1.0, ")

  # opacity(0) → opacity(0.0), opacity(1) → opacity(1.0)
  content.gsub!(/\.opacity\(0\)/, ".opacity(0.0)")
  content.gsub!(/\.opacity\(1\)/, ".opacity(1.0)")

  # frame patterns: frame(width: 0, height: 0)
  content.gsub!(/frame\(width:\s*(\d+)(?!\.\d)/) do
    val = $1
    "frame(width: #{val}.0"
  end
  content.gsub!(/frame\(height:\s*(\d+)(?!\.\d)/) do
    val = $1
    "frame(height: #{val}.0"
  end
  # But don't double-add .0
  content.gsub!(/(\d+)\.0\.0/, '\1.0')

  if content != original
    File.write(path, content)
    $changes[:int_fixed] << path.sub("#{SRC_ROOT}/", "")
  end
end

Dir.glob("#{SRC_ROOT}/**/*.swift").each { |f| fix_int_literals(f) }

# ============================================================
# Summary
# ============================================================

puts "=" * 60
puts "Skip Transpilation Fix Summary"
puts "=" * 60
puts ""
puts "Phase 1 - Files wrapped in #if !SKIP: #{$changes[:wrapped].size}"
$changes[:wrapped].each { |f| puts "  #{f}" }
puts ""
puts "Phase 2 - ultraThinMaterial fixed: #{$changes[:material_fixed].size}"
$changes[:material_fixed].each { |f| puts "  #{f}" }
puts ""
puts "Phase 3 - LiquidGlass→DesignSystem: #{$changes[:liquid_glass_fixed].size}"
$changes[:liquid_glass_fixed].each { |f| puts "  #{f}" }
puts ""
puts "Phase 4 - Int→Double literals: #{$changes[:int_fixed].size}"
$changes[:int_fixed].each { |f| puts "  #{f}" }
puts ""
puts "Total files modified: #{($changes.values.flatten.uniq).size}"
