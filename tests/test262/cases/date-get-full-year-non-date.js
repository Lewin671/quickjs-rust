// Derived from: test/built-ins/Date/prototype/getFullYear/this-value-non-date.js
var getFullYear = Date.prototype.getFullYear;

if (typeof getFullYear !== "function") { throw; }

var caught = false;
try {
  getFullYear.call({});
} catch (error) {
  caught = error instanceof TypeError;
}
if (!caught) { throw; }
