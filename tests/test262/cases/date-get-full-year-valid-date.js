// Derived from: test/built-ins/Date/prototype/getFullYear/this-value-valid-date.js
if (Date.prototype.getFullYear.length !== 0) { throw; }
if (new Date(2016, 0).getFullYear() !== 2016) { throw; }
if (new Date(2016, 0, 1, 0, 0, 0, -1).getFullYear() !== 2015) { throw; }
if (new Date(2016, 11, 31, 23, 59, 59, 999).getFullYear() !== 2016) { throw; }
if (new Date(2016, 11, 31, 23, 59, 59, 1000).getFullYear() !== 2017) { throw; }
