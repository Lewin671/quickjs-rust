// Derived from: test/built-ins/Date/parse/time-value-maximum-range.js
var minDateStr = "-271821-04-20T00:00:00.000Z";
var minDate = new Date(-8640000000000000);
if (minDate.toISOString() !== minDateStr) { throw; }
if (Date.parse(minDateStr) !== minDate.valueOf()) { throw; }

var maxDateStr = "+275760-09-13T00:00:00.000Z";
var maxDate = new Date(8640000000000000);
if (maxDate.toISOString() !== maxDateStr) { throw; }
if (Date.parse(maxDateStr) !== maxDate.valueOf()) { throw; }

if (!Number.isNaN(Date.parse("-271821-04-19T23:59:59.999Z"))) { throw; }
if (!Number.isNaN(Date.parse("+275760-09-13T00:00:00.001Z"))) { throw; }
