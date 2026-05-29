// Derived from: test/built-ins/Date/prototype/toUTCString/format.js
if (Date.prototype.toUTCString.length !== 0) { throw; }
if (new Date("1970-01-02T03:04:05.006Z").toUTCString() !== "Fri, 02 Jan 1970 03:04:05 GMT") { throw; }
if (new Date("0020-01-01T00:00:00Z").toUTCString() !== "Wed, 01 Jan 0020 00:00:00 GMT") { throw; }
if (new Date(NaN).toUTCString() !== "Invalid Date") { throw; }
