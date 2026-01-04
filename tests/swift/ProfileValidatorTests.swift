//
//  ProfileValidatorTests.swift
//  FoodshareCoreTests
//

@testable import FoodshareCore
import Testing

@Suite("ProfileValidator Tests")
struct ProfileValidatorTests {
    let validator = ProfileValidator()

    @Test("Valid nickname passes validation")
    func validNickname() {
        let result = validator.validate(nickname: "JohnDoe", bio: nil)
        #expect(result.isValid)
        #expect(result.errors.isEmpty)
    }

    @Test("Short nickname fails validation")
    func shortNickname() {
        let result = validator.validate(nickname: "A", bio: nil)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("2") == true)
    }

    @Test("Long nickname fails validation")
    func longNickname() {
        let longName = String(repeating: "a", count: 51)
        let result = validator.validate(nickname: longName, bio: nil)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("50") == true)
    }

    @Test("Empty nickname is allowed")
    func emptyNickname() {
        let result = validator.validate(nickname: "", bio: nil)
        #expect(result.isValid)
    }

    @Test("Nil nickname is allowed")
    func nilNickname() {
        let result = validator.validate(nickname: nil, bio: nil)
        #expect(result.isValid)
    }

    @Test("Valid bio passes validation")
    func validBio() {
        let result = validator.validate(nickname: nil, bio: "I love sharing food!")
        #expect(result.isValid)
    }

    @Test("Long bio fails validation")
    func longBio() {
        let longBio = String(repeating: "a", count: 301)
        let result = validator.validate(nickname: nil, bio: longBio)
        #expect(!result.isValid)
        #expect(result.firstError?.contains("300") == true)
    }

    @Test("Combined validation")
    func combinedValidation() {
        let result = validator.validate(nickname: "FoodLover", bio: "Sharing is caring!")
        #expect(result.isValid)
        #expect(result.errors.isEmpty)
    }
}
