//
//  ReviewValidatorTests.swift
//  FoodshareCoreTests
//

@testable import FoodshareCore
import Testing

@Suite("ReviewValidator Tests")
struct ReviewValidatorTests {
    let validator = ReviewValidator()

    @Test("Valid rating passes validation")
    func validRating() {
        for rating in 1...5 {
            let result = validator.validate(rating: rating)
            #expect(result.isValid, "Rating \(rating) should be valid")
        }
    }

    @Test("Rating below minimum fails validation")
    func ratingTooLow() {
        let result = validator.validate(rating: 0)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("between") == true)
    }

    @Test("Rating above maximum fails validation")
    func ratingTooHigh() {
        let result = validator.validate(rating: 6)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("between") == true)
    }

    @Test("Valid comment passes validation")
    func validComment() {
        let result = validator.validate(rating: 5, comment: "Great experience!")
        #expect(result.isValid)
        #expect(result.errors.isEmpty)
    }

    @Test("Long comment fails validation")
    func commentTooLong() {
        let longComment = String(repeating: "a", count: 501)
        let result = validator.validate(rating: 5, comment: longComment)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("500") == true)
    }

    @Test("Nil comment passes validation")
    func nilComment() {
        let result = validator.validate(rating: 4, comment: nil)
        #expect(result.isValid)
    }

    @Test("Empty comment passes validation")
    func emptyComment() {
        let result = validator.validate(rating: 3, comment: "")
        #expect(result.isValid)
    }
}
