// Derived from: test/built-ins/Date/prototype/setUTCMonth/length.js
if (Date.prototype.setUTCMonth.length !== 2) { throw; }

var month = new Date("1970-01-31T03:04:05.006Z");
var monthResult = month.setUTCMonth(1);
if (monthResult !== month.getTime()) { throw; }
if (month.toISOString() !== "1970-03-03T03:04:05.006Z") { throw; }

var date = new Date("1970-01-31T03:04:05.006Z");
var dateResult = date.setUTCMonth(1, 1);
if (dateResult !== date.getTime()) { throw; }
if (date.toISOString() !== "1970-02-01T03:04:05.006Z") { throw; }

var underflow = new Date("1970-01-31T03:04:05.006Z");
underflow.setUTCMonth(-1);
if (underflow.toISOString() !== "1969-12-31T03:04:05.006Z") { throw; }

var invalid = new Date(NaN);
if (!Number.isNaN(invalid.setUTCMonth(6, 7))) { throw; }
if (!Number.isNaN(invalid.getTime())) { throw; }

if (!Number.isNaN(new Date(0).setUTCMonth(undefined, 1))) { throw; }
