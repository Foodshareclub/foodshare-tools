//
//  TextSanitizerTests.swift
//  FoodshareCoreTests
//

@testable import FoodshareCore
import Testing

@Suite("TextSanitizer Tests")
struct TextSanitizerTests {

    // MARK: - Sanitize

    @Test("Trims whitespace")
    func trimsWhitespace() {
        let result = TextSanitizer.sanitize("  hello world  ")
        #expect(result == "hello world")
    }

    @Test("Normalizes line endings")
    func normalizesLineEndings() {
        let result = TextSanitizer.sanitize("line1\r\nline2\rline3")
        #expect(result == "line1\nline2\nline3")
    }

    @Test("Collapses multiple spaces")
    func collapsesSpaces() {
        let result = TextSanitizer.sanitize("hello    world")
        #expect(result == "hello world")
    }

    @Test("Collapses multiple newlines")
    func collapsesNewlines() {
        let result = TextSanitizer.sanitize("para1\n\n\n\npara2")
        #expect(result == "para1\n\npara2")
    }

    // MARK: - HTML Escaping

    @Test("Escapes ampersand")
    func escapesAmpersand() {
        let result = TextSanitizer.escapeHTML("A & B")
        #expect(result == "A &amp; B")
    }

    @Test("Escapes less than")
    func escapesLessThan() {
        let result = TextSanitizer.escapeHTML("a < b")
        #expect(result == "a &lt; b")
    }

    @Test("Escapes greater than")
    func escapesGreaterThan() {
        let result = TextSanitizer.escapeHTML("a > b")
        #expect(result == "a &gt; b")
    }

    @Test("Escapes quotes")
    func escapesQuotes() {
        let result = TextSanitizer.escapeHTML("Say \"hello\"")
        #expect(result == "Say &quot;hello&quot;")
    }

    @Test("Escapes single quotes")
    func escapesSingleQuotes() {
        let result = TextSanitizer.escapeHTML("It's me")
        #expect(result == "It&#39;s me")
    }

    @Test("Escapes XSS attempt")
    func escapesXSS() {
        let result = TextSanitizer.escapeHTML("<script>alert('xss')</script>")
        #expect(result == "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;")
    }

    // MARK: - HTML Stripping

    @Test("Strips simple HTML tags")
    func stripsSimpleTags() {
        let result = TextSanitizer.stripHTML("<b>bold</b> text")
        #expect(result == "bold text")
    }

    @Test("Strips tags with attributes")
    func stripsTagsWithAttributes() {
        let result = TextSanitizer.stripHTML("<a href=\"http://example.com\">link</a>")
        #expect(result == "link")
    }

    @Test("Strips self-closing tags")
    func stripsSelfClosingTags() {
        let result = TextSanitizer.stripHTML("Hello<br/>World")
        #expect(result == "HelloWorld")
    }

    @Test("Decodes HTML entities after stripping")
    func decodesEntitiesAfterStripping() {
        let result = TextSanitizer.stripHTML("<p>A &amp; B</p>")
        #expect(result == "A & B")
    }

    // MARK: - URL Safety

    @Test("Detects JavaScript URL scheme")
    func detectsJavaScript() {
        #expect(TextSanitizer.containsDangerousURLScheme("javascript:alert(1)"))
    }

    @Test("Detects data URL scheme")
    func detectsDataScheme() {
        #expect(TextSanitizer.containsDangerousURLScheme("data:text/html,<script>"))
    }

    @Test("Safe URL returns false")
    func safeURLReturnsFalse() {
        #expect(!TextSanitizer.containsDangerousURLScheme("https://example.com"))
    }
}
