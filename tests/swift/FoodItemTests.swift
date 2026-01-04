//
//  FoodItemTests.swift
//  FoodshareCoreTests
//
//  Tests for FoodItem model
//

import XCTest
@testable import FoodshareCore

final class FoodItemTests: XCTestCase {

    func testIsAvailable() {
        let availableItem = createFoodItem(isActive: true, isArranged: false)
        XCTAssertTrue(availableItem.isAvailable)

        let arrangedItem = createFoodItem(isActive: true, isArranged: true)
        XCTAssertFalse(arrangedItem.isAvailable)

        let inactiveItem = createFoodItem(isActive: false, isArranged: false)
        XCTAssertFalse(inactiveItem.isAvailable)
    }

    func testStatus() {
        let availableItem = createFoodItem(isActive: true, isArranged: false)
        XCTAssertEqual(availableItem.status, .available)

        let arrangedItem = createFoodItem(isActive: true, isArranged: true)
        XCTAssertEqual(arrangedItem.status, .arranged)

        let inactiveItem = createFoodItem(isActive: false, isArranged: false)
        XCTAssertEqual(inactiveItem.status, .inactive)
    }

    func testDistanceDisplay() {
        let nearItem = createFoodItem(distanceMeters: 500)
        XCTAssertEqual(nearItem.distanceDisplay, "500m")

        let farItem = createFoodItem(distanceMeters: 2500)
        XCTAssertEqual(farItem.distanceDisplay, "2.5km")

        let noDistanceItem = createFoodItem(distanceMeters: nil)
        XCTAssertNil(noDistanceItem.distanceDisplay)
    }

    func testDistanceKm() {
        let item = createFoodItem(distanceMeters: 1500)
        XCTAssertEqual(item.distanceKm, 1.5)
    }

    func testHasLocation() {
        let itemWithLocation = createFoodItem(latitude: 37.7749, longitude: -122.4194)
        XCTAssertTrue(itemWithLocation.hasLocation)

        let itemWithoutLocation = createFoodItem(latitude: nil, longitude: nil)
        XCTAssertFalse(itemWithoutLocation.hasLocation)
    }

    func testDisplayAddress() {
        // When stripped address is available, use it
        let itemWithStripped = createFoodItem()
        XCTAssertEqual(itemWithStripped.displayAddress, "Test St")

        // When stripped address is nil, fall back to full address
        let itemWithoutStripped = createFoodItemWithoutStrippedAddress()
        XCTAssertEqual(itemWithoutStripped.displayAddress, "123 Test St")
    }

    // MARK: - Helper

    private func createFoodItem(
        isActive: Bool = true,
        isArranged: Bool = false,
        distanceMeters: Double? = nil,
        latitude: Double? = 37.7749,
        longitude: Double? = -122.4194
    ) -> FoodItem {
        FoodItem(
            id: 1,
            profileId: UUID(),
            postName: "Test Food",
            postDescription: "Test description",
            postType: "food",
            pickupTime: nil,
            availableHours: nil,
            postAddress: "123 Test St",
            postStrippedAddress: "Test St",
            latitude: latitude,
            longitude: longitude,
            images: nil,
            isActive: isActive,
            isArranged: isArranged,
            postArrangedTo: nil,
            postArrangedAt: nil,
            postViews: 0,
            postLikeCounter: nil,
            hasPantry: nil,
            foodStatus: nil,
            network: nil,
            website: nil,
            donation: nil,
            donationRules: nil,
            categoryId: nil,
            createdAt: Date(),
            updatedAt: Date(),
            distanceMeters: distanceMeters
        )
    }

    private func createFoodItemWithoutStrippedAddress() -> FoodItem {
        FoodItem(
            id: 1,
            profileId: UUID(),
            postName: "Test Food",
            postDescription: "Test description",
            postType: "food",
            pickupTime: nil,
            availableHours: nil,
            postAddress: "123 Test St",
            postStrippedAddress: nil,
            latitude: 37.7749,
            longitude: -122.4194,
            images: nil,
            isActive: true,
            isArranged: false,
            postArrangedTo: nil,
            postArrangedAt: nil,
            postViews: 0,
            postLikeCounter: nil,
            hasPantry: nil,
            foodStatus: nil,
            network: nil,
            website: nil,
            donation: nil,
            donationRules: nil,
            categoryId: nil,
            createdAt: Date(),
            updatedAt: Date(),
            distanceMeters: nil
        )
    }
}
