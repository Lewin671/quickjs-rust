// Derived from: test/built-ins/Date/prototype/getUTCFullYear/this-value-valid-date.js
var date = new Date("2016-12-31T23:59:59.999Z");
if (Date.prototype.getUTCFullYear.length !== 0) { throw; }
if (date.getUTCFullYear() !== 2016) { throw; }
if (date.getUTCMonth() !== 11) { throw; }
if (date.getUTCDate() !== 31) { throw; }
if (date.getUTCDay() !== 6) { throw; }
if (date.getUTCHours() !== 23) { throw; }
if (date.getUTCMinutes() !== 59) { throw; }
if (date.getUTCSeconds() !== 59) { throw; }
if (date.getUTCMilliseconds() !== 999) { throw; }
if (!Number.isNaN(new Date(NaN).getUTCFullYear())) { throw; }
