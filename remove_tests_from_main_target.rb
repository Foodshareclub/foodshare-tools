#!/usr/bin/env ruby
# Removes __tests__ files from the main FoodShare target (they belong in the test target).

require 'xcodeproj'

PROJECT_PATH = '/Users/organic/dev/work/foodshare/foodshare-ios/FoodShare.xcodeproj'

project = Xcodeproj::Project.open(PROJECT_PATH)
main_target = project.targets.find { |t| t.name == 'FoodShare' }

abort "ERROR: No 'FoodShare' target found" unless main_target

# Find test files that were incorrectly added to the main target
test_files_to_remove = []
main_target.source_build_phase.files.each do |build_file|
  next unless build_file.file_ref
  path = build_file.file_ref.path || ''
  name = build_file.file_ref.name || ''
  if path.include?('__tests__') || name.include?('Tests.swift')
    test_files_to_remove << build_file
    puts "  REMOVING from main target: #{name} (#{path})"
  end
end

test_files_to_remove.each do |bf|
  main_target.source_build_phase.files.delete(bf)
end

project.save

puts ""
puts "Done! Removed #{test_files_to_remove.count} test files from main target."
