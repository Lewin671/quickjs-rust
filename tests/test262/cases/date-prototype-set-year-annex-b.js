// Derived from: test/annexB/built-ins/Date/prototype/setYear/this-time-valid.js
// Derived from: test/annexB/built-ins/Date/prototype/setYear/this-time-nan.js
// Derived from: test/annexB/built-ins/Date/prototype/setYear/year-number-relative.js
// Derived from: test/annexB/built-ins/Date/prototype/setYear/year-number-absolute.js
// Derived from: test/annexB/built-ins/Date/prototype/setYear/year-nan.js

if (Date.prototype.setYear.length !== 1) {
  throw "Date.prototype.setYear length should be 1";
}

var date = new Date(1970, 1, 2, 3, 4, 5);
var expected = new Date(1971, 1, 2, 3, 4, 5).valueOf();
if (date.setYear(71) !== expected || date.valueOf() !== expected) {
  throw "Date.prototype.setYear should preserve local fields";
}

date = new Date({});
expected = new Date(1971, 0).valueOf();
if (date.setYear(71) !== expected || date.valueOf() !== expected) {
  throw "Date.prototype.setYear should use +0 for invalid receivers";
}

date = new Date(1970, 0);
date.setYear(50.999999);
if (date.getFullYear() !== 1950) {
  throw "Date.prototype.setYear should offset relative years";
}

date = new Date(1970, 0);
date.setYear(100);
if (date.getFullYear() !== 100) {
  throw "Date.prototype.setYear should preserve absolute years";
}

date = new Date(0);
if (!Number.isNaN(date.setYear()) || !Number.isNaN(date.valueOf())) {
  throw "Date.prototype.setYear should store NaN for missing year";
}

date = new Date(0);
if (!Number.isNaN(date.setYear("not a number")) || !Number.isNaN(date.valueOf())) {
  throw "Date.prototype.setYear should store NaN for NaN year";
}

try {
  Date.prototype.setYear.call({}, 0);
  throw "Date.prototype.setYear should reject non-Date receivers";
} catch (error) {
  if (!(error instanceof TypeError)) {
    throw error;
  }
}
