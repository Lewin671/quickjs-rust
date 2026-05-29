// Derived from: test/built-ins/Date/prototype/setUTCFullYear/length.js
if (Date.prototype.setUTCFullYear.length !== 3) { throw; }

var date = new Date("1970-06-02T03:04:05.006Z");
var result = date.setUTCFullYear(2000);
if (result !== date.getTime()) { throw; }
if (date.toISOString() !== "2000-06-02T03:04:05.006Z") { throw; }

var parts = new Date("1970-01-02T03:04:05.006Z");
parts.setUTCFullYear(1, 1, 3);
if (parts.toISOString() !== "0001-02-03T03:04:05.006Z") { throw; }

var invalid = new Date(NaN);
var invalidResult = invalid.setUTCFullYear(1);
if (invalidResult !== invalid.getTime()) { throw; }
if (invalid.toISOString() !== "0001-01-01T00:00:00.000Z") { throw; }

var overflow = new Date(0);
if (!Number.isNaN(overflow.setUTCFullYear(275760, 8, 14))) { throw; }
if (!Number.isNaN(overflow.getTime())) { throw; }
