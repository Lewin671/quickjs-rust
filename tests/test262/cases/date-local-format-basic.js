// Derived from: test/built-ins/Date/prototype/toString/format.js
if (Date.prototype.getTimezoneOffset.length !== 0) { throw; }
if (Date.prototype.toDateString.length !== 0) { throw; }
if (Date.prototype.toString.length !== 0) { throw; }
if (Date.prototype.toTimeString.length !== 0) { throw; }

function hasDateShape(value) {
  return value.length === 15 && value.charAt(3) === " " && value.charAt(7) === " " && value.charAt(10) === " ";
}
function hasTimeShape(value) {
  return value.length >= 17 && value.charAt(2) === ":" && value.charAt(5) === ":" && value.indexOf(" GMT") === 8;
}
function hasLocalStringShape(value) {
  return value.length >= 33 && hasDateShape(value.substring(0, 15)) && hasTimeShape(value.substring(16));
}

if (!hasDateShape(new Date(0).toDateString())) { throw; }
if (!hasDateShape(new Date("0020-01-01T00:00:00Z").toDateString())) { throw; }

if (!hasTimeShape(new Date(0).toTimeString())) { throw; }

if (!hasLocalStringShape(new Date(0).toString())) { throw; }
if (!hasLocalStringShape(new Date("0020-01-01T00:00:00Z").toString())) { throw; }
if (!hasLocalStringShape(Date())) { throw; }

if (new Date(NaN).toString() !== "Invalid Date") { throw; }
if (new Date(NaN).toDateString() !== "Invalid Date") { throw; }
if (new Date(NaN).toTimeString() !== "Invalid Date") { throw; }
if (typeof new Date(0).getTimezoneOffset() !== "number") { throw; }
if (!Number.isNaN(new Date(NaN).getTimezoneOffset())) { throw; }
