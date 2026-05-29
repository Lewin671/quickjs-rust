// Derived from: test/built-ins/Date/prototype/setUTCDate/length.js
if (Date.prototype.setUTCDate.length !== 1) { throw; }

var date = new Date("1970-01-31T03:04:05.006Z");
var dateResult = date.setUTCDate(1);
if (dateResult !== date.getTime()) { throw; }
if (date.toISOString() !== "1970-01-01T03:04:05.006Z") { throw; }

var overflow = new Date("1970-01-31T03:04:05.006Z");
var overflowResult = overflow.setUTCDate(32);
if (overflowResult !== overflow.getTime()) { throw; }
if (overflow.toISOString() !== "1970-02-01T03:04:05.006Z") { throw; }

var underflow = new Date("1970-01-31T03:04:05.006Z");
var underflowResult = underflow.setUTCDate(0);
if (underflowResult !== underflow.getTime()) { throw; }
if (underflow.toISOString() !== "1969-12-31T03:04:05.006Z") { throw; }

var invalid = new Date(NaN);
if (!Number.isNaN(invalid.setUTCDate(1))) { throw; }
if (!Number.isNaN(invalid.getTime())) { throw; }

var nan = new Date(0);
if (!Number.isNaN(nan.setUTCDate(undefined))) { throw; }
if (!Number.isNaN(nan.getTime())) { throw; }
