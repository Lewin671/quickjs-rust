// Derived from: test/built-ins/Date/parse/zero.js
if (Date.parse("1970-01-02T03:04:05.006Z") !== 97445006) { throw; }
if (Date.UTC(1970, 0, 2, 3, 4, 5, 6) !== 97445006) { throw; }
if (new Date("1970-01-02T03:04:05.006Z").toISOString() !== "1970-01-02T03:04:05.006Z") { throw; }
