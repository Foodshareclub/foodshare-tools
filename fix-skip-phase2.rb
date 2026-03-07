#!/usr/bin/env ruby
# Phase 2: More aggressive Skip transpilation fixes
# Wraps remaining data layer files and fixes UI patterns

APP_ROOT = "/Users/organic/dev/work/foodshare/foodshare-app"
SRC_ROOT = "#{APP_ROOT}/Sources/FoodShare"

$changes = { wrapped: [], fixed: [] }

# ============================================================
# Helper: Wrap entire file in #if !SKIP (handle existing partial guards)
# ============================================================

def force_wrap_file(path)
  return unless File.exist?(path)
  content = File.read(path)

  # Already fully wrapped? Check if first non-comment line is #if !SKIP
  # AND last non-blank line is #endif
  lines = content.lines
  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }

  return if first_code && last_code &&
            lines[first_code]&.strip == "#if !SKIP" &&
            lines[last_code]&.strip == "#endif"

  # Remove existing partial #if !SKIP / #endif pairs for import CoreData
  content = content.gsub(/^#if !SKIP\nimport CoreData\n#endif\n/, "import CoreData\n")

  # Find header end
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
# PHASE 2A: Force-wrap remaining Supabase repos
# ============================================================

# Repos that had partial guards (import CoreData only)
repos_to_wrap = Dir.glob("#{SRC_ROOT}/**/Supabase*Repository.swift") +
                Dir.glob("#{SRC_ROOT}/**/Supabase*Repository*.swift")

repos_to_wrap.each { |f| force_wrap_file(f) }

# ============================================================
# PHASE 2B: Force-wrap remaining service/data files
# ============================================================

# AuthenticationService and other top error services
service_files = %w[
  Core/Services/AuthenticationService.swift
  Core/Services/PostActivityService.swift
  Core/Services/PostEngagementService.swift
  Core/Services/PostViewService.swift
  Core/Services/ShareService.swift
  Core/Services/StoreKitService.swift
  Core/Services/ErrorRecoveryService.swift
  Core/Services/ImagePrefetchService.swift
  Core/Services/AppLockService.swift
  Core/Services/WidgetDataProvider.swift
  Core/Services/ChallengeEngagementService.swift
  Core/Services/ForumEngagementService.swift
  Core/Services/NewsletterService.swift
  Core/Services/EmailPreferencesService.swift
  Core/Localization/EnhancedTranslationService.swift
  Core/Database/RealtimeService.swift
  Core/Database/StorageService.swift
  Core/Notifications/PushNotificationService.swift
  Core/Networking/APIClient.swift
  Core/Networking/NetworkService.swift
  Core/Networking/ResilientNetworkService.swift
  Core/Networking/MockNetworkService.swift
  Core/Cache/CachingService.swift
  Core/Security/AppAttestService.swift
  Core/Security/DeviceCheckService.swift
  Core/Security/PrivacyProtectionService.swift
  Core/Security/SecurityScoreService.swift
  Core/Error/ErrorReportingService.swift
  Core/Compliance/GDPRExportService.swift
  Core/Location/IPGeolocationService.swift
  Core/Location/MockLocationService.swift
  Core/Storage/CacheService.swift
  Features/Settings/Domain/Services/SettingsBackupService.swift
  Features/Map/Services/MapPreferencesService.swift
  Features/Messaging/Domain/Services/ChatPresenceService.swift
  Features/Forum/Domain/Services/ForumRealtimeService.swift
  Features/Feed/Domain/Services/FeedDataService.swift
  Features/Feedback/Data/SupabaseFeedbackRepository.swift
  Features/Reports/Data/SupabaseReportRepository.swift
]

service_files.each { |f| force_wrap_file("#{SRC_ROOT}/#{f}") }

# ============================================================
# PHASE 2C: Wrap ViewModels that heavily reference iOS services
# ============================================================

vm_files = %w[
  Features/Notifications/Presentation/ViewModels/NotificationPreferencesViewModel.swift
  Features/Notifications/Presentation/ViewModels/NotificationCenterViewModel.swift
  Features/Notifications/Presentation/ViewModels/NotificationsViewModel.swift
  Features/Feed/Presentation/ViewModels/FeedViewModel.swift
  Features/Search/Presentation/ViewModels/SearchViewModel.swift
  Features/Messaging/Presentation/ViewModels/MessagingViewModel.swift
  Features/Messaging/Presentation/ViewModels/ChatViewModel.swift
  Features/Challenges/Presentation/ViewModels/ChallengesViewModel.swift
  Features/Profile/Presentation/ViewModels/ProfileViewModel.swift
  Features/Profile/Presentation/ViewModels/EditProfileViewModel.swift
  Features/Forum/Presentation/ViewModels/ForumViewModel.swift
  Features/Listing/Presentation/ViewModels/CreateListingViewModel.swift
  Features/Authentication/Presentation/ViewModels/AuthViewModel.swift
  Features/Feedback/Presentation/ViewModels/FeedbackViewModel.swift
  Features/Activity/Presentation/ViewModels/ActivityViewModel.swift
  Features/Settings/Presentation/ViewModels/SettingsViewModel.swift
  Features/CommunityFridges/Presentation/ViewModels/CommunityFridgesViewModel.swift
  Features/Admin/Presentation/ViewModels/AdminViewModel.swift
]

vm_files.each { |f| force_wrap_file("#{SRC_ROOT}/#{f}") }

# ============================================================
# PHASE 2D: Wrap remaining data/domain files that use iOS APIs
# ============================================================

data_files = %w[
  Core/Sync/ConflictResolution.swift
  Core/Sync/SyncableEntities.swift
  Core/State/AppStateRestoration.swift
  Core/State/AppStateManager.swift
  Core/Crash/CrashReporter.swift
  Core/Extensions/Color+Extensions.swift
  Core/Utilities/InputSanitizer.swift
  Core/FeatureFlags/FeatureFlagManager.swift
  Core/Analytics/AnalyticsEventCatalog.swift
  Core/Network/NetworkConfiguration.swift
  Core/Networking/CircuitBreaker.swift
  Core/Security/PayloadEncryption.swift
  Core/Security/CertificatePinning.swift
  Core/Location/UserLocationOverrideManager.swift
  Core/Location/IPGeolocationRetryPolicy.swift
  Core/Performance/FrameRateMonitor.swift
  Core/Error/RetryPolicy.swift
  Features/Notifications/Data/Repositories/SupabaseNotificationPreferencesRepository.swift
  Features/Notifications/Domain/Models/UserNotification.swift
  Features/Notifications/Domain/Models/NotificationPreferencesRepository.swift
  Features/Feed/Domain/Repositories/FeedRepository.swift
  Features/Feed/Domain/UseCases/FetchNearbyItemsUseCase.swift
  Features/Feed/Domain/UseCases/SearchFoodItemsUseCase.swift
  Features/Messaging/Domain/Repositories/MessagingRepository.swift
  Features/Listing/Domain/Repositories/ListingRepository.swift
  Features/Forum/Domain/Repositories/ForumRepository.swift
  Features/Profile/Domain/Models/AuthUser.swift
  Features/Settings/Presentation/Views/PrivacySettingsView.swift
  Features/Settings/Presentation/Views/ScreenRecordingWarningView.swift
  Features/Settings/Presentation/Views/EnterpriseNotificationPreferencesView.swift
  Features/Notifications/Presentation/Views/PushNotificationSender.swift
  Features/Activity/Domain/Models/PostActivityTimelineView.swift
  App/MainTab/MainTabView.swift
  App/Router/NavigationCoordinator.swift
  App/Router/SheetCoordinator.swift
  App/DeepLinks/DeepLinkChallengeView.swift
  App/DeepLinks/DeepLinkForumPostView.swift
  App/DeepLinks/DeepLinkListingView.swift
  App/DeepLinks/DeepLinkProfileView.swift
  App/State/AppState.swift
  App/State/AppConfiguration.swift
  App/Logging/LogEntry.swift
  App/Networking/ResendEmailClient.swift
  App/Networking/RequestContext.swift
  Features/Challenges/Domain/Models/Challenge.swift
  Features/Forum/Domain/Models/ForumPost.swift
  Features/Forum/Domain/Models/ForumCategory.swift
  Features/Forum/Domain/Models/ForumBadge.swift
  Features/Forum/Domain/Models/ForumUserStats.swift
  Features/Forum/Domain/Models/ForumPoll.swift
  Features/Feed/Domain/Models/FoodItem.swift
  Features/CommunityFridges/Domain/Models/CommunityFridge.swift
]

data_files.each { |f| force_wrap_file("#{SRC_ROOT}/#{f}") }

# ============================================================
# PHASE 2E: Wrap remaining View files with heavy iOS dependencies
# ============================================================

view_files_to_wrap = %w[
  Features/Notifications/Presentation/Views/NotificationCenter/NotificationCenterOverlay.swift
  Features/Notifications/Presentation/Views/NotificationsView.swift
  Features/Notifications/Presentation/Views/NotificationCenter/NotificationDropdown.swift
  Features/Notifications/Presentation/Views/NotificationCenter/NotificationRowCompact.swift
  Features/Settings/Presentation/Views/EnterpriseNotificationSettingsView.swift
  Features/Settings/Presentation/Views/NotificationsSettingsView.swift
  Features/Settings/Presentation/Views/GlobalSettingsSection.swift
  Features/Settings/Presentation/Components/Molecules/CategoryPreferenceCard.swift
  Features/Settings/Presentation/Components/Molecules/DNDStatusCard.swift
  Features/Settings/Presentation/Components/Molecules/NotificationPreferenceRow.swift
  Features/Settings/Presentation/Components/Molecules/QuietHoursCard.swift
  Features/Settings/Presentation/Components/Atoms/NotificationIcon.swift
  Features/Settings/Presentation/Components/Atoms/NotificationToggle.swift
  Features/Settings/Presentation/Components/Atoms/FrequencyBadge.swift
  Features/Settings/Presentation/Components/Atoms/NotificationBadge.swift
  Features/Settings/Presentation/Components/Atoms/NotificationDot.swift
  Features/Settings/Presentation/Components/Atoms/StatusIndicator.swift
  Features/Settings/Presentation/Components/Organisms/ScheduleSection.swift
  Features/Settings/Presentation/Components/Organisms/CategoryPreferencesSection.swift
  Features/Settings/Presentation/Views/EmailPreferencesView.swift
  Features/Activity/Presentation/Views/ActivityFeedView.swift
  Features/Activity/Presentation/Views/PostActivityTimelineView.swift
  Features/Admin/Presentation/Views/AdminDashboardView.swift
  Features/Admin/Presentation/Views/AdminUsersView.swift
  Features/Admin/Presentation/Views/AdminModerationView.swift
  Features/Messaging/Presentation/Views/MessagingView.swift
  Features/Messaging/Presentation/Views/ChannelHeader.swift
  Features/Messaging/Presentation/Views/ArrangementView.swift
  App/EntryPoints/FoodShareEntryPoints.swift
  App/Root/RootView.swift
  App/Environment/Environment.swift
]

view_files_to_wrap.each { |f| force_wrap_file("#{SRC_ROOT}/#{f}") }

# ============================================================
# PHASE 2F: Fix remaining .ultraThinMaterial patterns
# (some escaped first pass — inline usage, etc.)
# ============================================================

Dir.glob("#{SRC_ROOT}/**/*.swift").each do |path|
  content = File.read(path)
  next unless content.include?("ultraThinMaterial")

  # Skip if the whole file is already wrapped in #if !SKIP
  lines = content.lines
  first_code = lines.index { |l| l.strip != "" && !l.strip.start_with?("//") }
  last_code = lines.rindex { |l| l.strip != "" }
  next if first_code && last_code &&
          lines[first_code]&.strip == "#if !SKIP" &&
          lines[last_code]&.strip == "#endif"

  original = content.dup

  # Inline .ultraThinMaterial not caught by Phase 1
  # Replace remaining standalone .ultraThinMaterial with Color fallback
  # This handles cases like: .fill(.ultraThinMaterial) on same line as other code
  content.gsub!(/\.ultraThinMaterial/) do |match|
    "Color.DesignSystem.glassSurface.opacity(0.15) /* ultraThinMaterial fallback */"
  end

  if content != original
    File.write(path, content)
    $changes[:fixed] << path.sub("#{SRC_ROOT}/", "")
  end
end

# ============================================================
# Summary
# ============================================================

puts "=" * 60
puts "Phase 2 Fix Summary"
puts "=" * 60
puts ""
puts "Files force-wrapped in #if !SKIP: #{$changes[:wrapped].size}"
$changes[:wrapped].each { |f| puts "  #{f}" }
puts ""
puts "Files with ultraThinMaterial inline fixes: #{$changes[:fixed].size}"
$changes[:fixed].each { |f| puts "  #{f}" }
puts ""
puts "Total files modified: #{($changes[:wrapped] + $changes[:fixed]).uniq.size}"
