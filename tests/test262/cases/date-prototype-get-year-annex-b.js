// Derived from: test/annexB/built-ins/Date/prototype/getYear/return-value.js
// Derived from: test/annexB/built-ins/Date/prototype/getYear/nan.js

if (new Date(1899, 0).getYear() !== -1) {
  throw "Date.prototype.getYear should offset years before 1900";
}

if (new Date(1900, 0).getYear() !== 0) {
  throw "Date.prototype.getYear should return zero for 1900";
}

if (new Date(1970, 0).getYear() !== 70) {
  throw "Date.prototype.getYear should offset years after 1900";
}

if (new Date(2000, 0).getYear() !== 100) {
  throw "Date.prototype.getYear should offset years after 1999";
}

if (!Number.isNaN(new Date(NaN).getYear())) {
  throw "Date.prototype.getYear should preserve NaN time values";
}
