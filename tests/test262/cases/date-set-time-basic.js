// Derived from: test/built-ins/Date/prototype/setTime/this-value-valid-date.js
if (Date.prototype.setTime.length !== 1) { throw; }
var date = new Date(0);
var result = date.setTime(97445006.9);
if (result !== 97445006) { throw; }
if (date.getTime() !== 97445006) { throw; }
if (date.toISOString() !== "1970-01-02T03:04:05.006Z") { throw; }
if (!Number.isNaN(date.setTime(NaN))) { throw; }
if (!Number.isNaN(date.getTime())) { throw; }
var overflow = new Date(0);
if (!Number.isNaN(overflow.setTime(8640000000000001))) { throw; }
if (!Number.isNaN(overflow.getTime())) { throw; }
