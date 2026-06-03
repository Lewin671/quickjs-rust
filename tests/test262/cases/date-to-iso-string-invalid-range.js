// Derived from: test/built-ins/Date/prototype/toISOString/15.9.5.43-0-13.js
// Derived from: test/built-ins/Date/prototype/toISOString/15.9.5.43-0-8.js
var caught = false;
try {
  new Date(8640000000000001).toISOString();
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }

caught = false;
try {
  new Date(1970, 0, -99999999, 0, -60, 0, -1).toISOString();
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }
