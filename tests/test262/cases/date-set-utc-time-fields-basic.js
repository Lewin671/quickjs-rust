// Derived from: test/built-ins/Date/prototype/setUTCHours/length.js
if (Date.prototype.setUTCHours.length !== 4) { throw; }
if (Date.prototype.setUTCMilliseconds.length !== 1) { throw; }
if (Date.prototype.setUTCMinutes.length !== 3) { throw; }
if (Date.prototype.setUTCSeconds.length !== 2) { throw; }

var hours = new Date("1970-01-02T03:04:05.006Z");
var hoursResult = hours.setUTCHours(10);
if (hoursResult !== 122645006) { throw; }
if (hours.toISOString() !== "1970-01-02T10:04:05.006Z") { throw; }

var hoursParts = new Date("1970-01-02T03:04:05.006Z");
var hoursPartsResult = hoursParts.setUTCHours(10, 11, 12, 13);
if (hoursPartsResult !== 123072013) { throw; }
if (hoursParts.toISOString() !== "1970-01-02T10:11:12.013Z") { throw; }

var hoursOverflow = new Date("1970-01-02T03:04:05.006Z");
var hoursOverflowResult = hoursOverflow.setUTCHours(24);
if (hoursOverflowResult !== 173045006) { throw; }
if (hoursOverflow.toISOString() !== "1970-01-03T00:04:05.006Z") { throw; }

var minutes = new Date("1970-01-02T03:04:05.006Z");
var minutesResult = minutes.setUTCMinutes(30, 31, 32);
if (minutesResult !== 99031032) { throw; }
if (minutes.toISOString() !== "1970-01-02T03:30:31.032Z") { throw; }

var seconds = new Date("1970-01-02T03:04:05.006Z");
var secondsResult = seconds.setUTCSeconds(40, 41);
if (secondsResult !== 97480041) { throw; }
if (seconds.toISOString() !== "1970-01-02T03:04:40.041Z") { throw; }

var milliseconds = new Date("1970-01-02T03:04:05.006Z");
var millisecondsResult = milliseconds.setUTCMilliseconds(500);
if (millisecondsResult !== 97445500) { throw; }
if (milliseconds.toISOString() !== "1970-01-02T03:04:05.500Z") { throw; }

var invalid = new Date(NaN);
if (!Number.isNaN(invalid.setUTCHours(1, 2, 3, 4))) { throw; }
if (!Number.isNaN(invalid.getTime())) { throw; }

var nan = new Date(0);
if (!Number.isNaN(nan.setUTCSeconds(undefined, 1))) { throw; }
if (!Number.isNaN(nan.getTime())) { throw; }

var noArgs = new Date(0);
if (!Number.isNaN(noArgs.setUTCMilliseconds())) { throw; }
if (!Number.isNaN(noArgs.getTime())) { throw; }
