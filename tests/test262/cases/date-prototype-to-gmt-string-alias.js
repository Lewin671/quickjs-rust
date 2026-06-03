// Derived from: test/annexB/built-ins/Date/prototype/toGMTString/value.js

if (Date.prototype.toGMTString !== Date.prototype.toUTCString) {
  throw "Date.prototype.toGMTString should be the initial toUTCString function object";
}

if (new Date("1970-01-02T03:04:05.006Z").toGMTString() !== "Fri, 02 Jan 1970 03:04:05 GMT") {
  throw "Date.prototype.toGMTString should format like toUTCString";
}
