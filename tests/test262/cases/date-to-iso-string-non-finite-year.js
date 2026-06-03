// Derived from: test/built-ins/Date/prototype/toISOString/15.9.5.43-0-14.js
// Derived from: test/built-ins/Date/prototype/toISOString/15.9.5.43-0-15.js
var caught = false;
try {
  new Date(-Infinity, 1, 70, 0, 0, 0).toISOString();
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }

caught = false;
try {
  new Date(Infinity, 1, 70, 0, 0, 0).toISOString();
} catch (error) {
  caught = error instanceof RangeError;
}
if (!caught) { throw; }
