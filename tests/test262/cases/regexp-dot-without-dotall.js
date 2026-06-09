// Derived from: test/built-ins/RegExp/dotall/without-dotall.js
// Derived from: test/built-ins/RegExp/dotall/without-dotall-unicode.js

function assertSameValue(actual, expected, message) {
  if (actual !== expected) {
    throw message;
  }
}

for (var i = 0; i < 2; i++) {
  var re = i === 0 ? /^.$/ : /^.$/m;
  assertSameValue(re.test("a"), true, "dot should match ordinary characters");
  assertSameValue(re.test("\u2027"), true, "dot should match non-line separators");
  assertSameValue(re.test("\u0085"), true, "dot should match next-line control");
  assertSameValue(re.test("\v"), true, "dot should match vertical tab");
  assertSameValue(re.test("\f"), true, "dot should match form feed");
  assertSameValue(re.test("\n"), false, "dot should reject line feed");
  assertSameValue(re.test("\r"), false, "dot should reject carriage return");
  assertSameValue(re.test("\u2028"), false, "dot should reject line separator");
  assertSameValue(re.test("\u2029"), false, "dot should reject paragraph separator");
}

for (var j = 0; j < 2; j++) {
  var unicodeRe = j === 0 ? /^.$/u : /^.$/um;
  assertSameValue(unicodeRe.test("\u2027"), true, "unicode dot should match non-line separators");
  assertSameValue(unicodeRe.test("\n"), false, "unicode dot should reject line feed");
  assertSameValue(unicodeRe.test("\r"), false, "unicode dot should reject carriage return");
  assertSameValue(unicodeRe.test("\u2028"), false, "unicode dot should reject line separator");
  assertSameValue(unicodeRe.test("\u2029"), false, "unicode dot should reject paragraph separator");
}
