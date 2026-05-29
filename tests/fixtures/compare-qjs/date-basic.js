(function () {
  var value = new Date("1970-01-02T03:04:05.006Z");
  var custom = new Date("1970-01-02T03:04:05.006Z");
  var mutable = new Date(0);
  var setResult = mutable.setTime(97445006.9);
  var utcYear = new Date("1970-06-02T03:04:05.006Z");
  var utcYearResult = utcYear.setUTCFullYear(2000);
  var utcYearParts = new Date("1970-01-02T03:04:05.006Z");
  utcYearParts.setUTCFullYear(1, 1, 3);
  var utcYearInvalid = new Date(NaN);
  var utcYearInvalidResult = utcYearInvalid.setUTCFullYear(1);
  var utcMonth = new Date("1970-01-31T03:04:05.006Z");
  var utcMonthResult = utcMonth.setUTCMonth(1);
  var utcMonthDate = new Date("1970-01-31T03:04:05.006Z");
  var utcMonthDateResult = utcMonthDate.setUTCMonth(1, 1);
  var utcMonthOverflow = new Date("1970-01-31T03:04:05.006Z");
  utcMonthOverflow.setUTCMonth(-1);
  var utcMonthInvalid = new Date(NaN);
  var utcMonthInvalidResult = utcMonthInvalid.setUTCMonth(6, 7);
  var utcDate = new Date("1970-01-31T03:04:05.006Z");
  var utcDateResult = utcDate.setUTCDate(1);
  var utcDateOverflow = new Date("1970-01-31T03:04:05.006Z");
  var utcDateOverflowResult = utcDateOverflow.setUTCDate(32);
  var utcDateUnderflow = new Date("1970-01-31T03:04:05.006Z");
  var utcDateUnderflowResult = utcDateUnderflow.setUTCDate(0);
  var utcDateInvalid = new Date(NaN);
  var utcDateInvalidResult = utcDateInvalid.setUTCDate(1);
  var utcHours = new Date("1970-01-02T03:04:05.006Z");
  var utcHoursResult = utcHours.setUTCHours(10);
  var utcHoursParts = new Date("1970-01-02T03:04:05.006Z");
  var utcHoursPartsResult = utcHoursParts.setUTCHours(10, 11, 12, 13);
  var utcMinutes = new Date("1970-01-02T03:04:05.006Z");
  var utcMinutesResult = utcMinutes.setUTCMinutes(30, 31, 32);
  var utcSeconds = new Date("1970-01-02T03:04:05.006Z");
  var utcSecondsResult = utcSeconds.setUTCSeconds(40, 41);
  var utcMilliseconds = new Date("1970-01-02T03:04:05.006Z");
  var utcMillisecondsResult = utcMilliseconds.setUTCMilliseconds(500);
  var utcNoArgs = new Date(0);
  var utcNoArgsResult = utcNoArgs.setUTCMilliseconds();
  function hasDateShape(value) {
    return value.length === 15 && value.charAt(3) === " " && value.charAt(7) === " " && value.charAt(10) === " ";
  }
  function hasTimeShape(value) {
    return value.length >= 17 && value.charAt(2) === ":" && value.charAt(5) === ":" && value.indexOf(" GMT") === 8;
  }
  function hasLocalStringShape(value) {
    return value.length >= 33 && hasDateShape(value.substring(0, 15)) && hasTimeShape(value.substring(16));
  }
  var invalid = new Date(0);
  custom.toISOString = function () { return "custom"; };
  return [
    typeof Date,
    Date.length,
    Date.parse.length,
    Date.UTC.length,
    Date.prototype.getTime.length,
    Date.prototype.getUTCFullYear.length,
    Date.prototype.toJSON.length,
    Date.prototype.setTime.length,
    Date.prototype.setUTCDate.length,
    Date.prototype.setUTCFullYear.length,
    Date.prototype.setUTCHours.length,
    Date.prototype.setUTCMilliseconds.length,
    Date.prototype.setUTCMinutes.length,
    Date.prototype.setUTCMonth.length,
    Date.prototype.setUTCSeconds.length,
    Date.prototype.getTimezoneOffset.length,
    Date.prototype.toDateString.length,
    Date.prototype.toString.length,
    Date.prototype.toTimeString.length,
    value.getTime(),
    value.valueOf(),
    value.toISOString(),
    value.toJSON(),
    value.toUTCString(),
    value.getUTCFullYear(),
    value.getUTCMonth(),
    value.getUTCDate(),
    value.getUTCDay(),
    value.getUTCHours(),
    value.getUTCMinutes(),
    value.getUTCSeconds(),
    value.getUTCMilliseconds(),
    Date.UTC(1970, 0, 2, 3, 4, 5, 6),
    Date.parse("1970-01-02T03:04:05.006Z"),
    new Date(0).toISOString(),
    new Date(8640000000000000).toISOString(),
    new Date(-8640000000000000).toISOString(),
    Date.parse("+275760-09-13T00:00:00.000Z"),
    Date.parse("-271821-04-20T00:00:00.000Z"),
    Number.isNaN(Date.parse("+275760-09-13T00:00:00.001Z")),
    new Date("0020-01-01T00:00:00Z").toUTCString(),
    JSON.stringify(value) === '"1970-01-02T03:04:05.006Z"',
    JSON.stringify({ when: value }) === '{"when":"1970-01-02T03:04:05.006Z"}',
    custom.toJSON(),
    JSON.stringify(custom) === '"custom"',
    setResult,
    mutable.getTime(),
    mutable.toISOString(),
    utcYearResult,
    utcYear.toISOString(),
    utcYearParts.toISOString(),
    utcYearInvalidResult,
    utcYearInvalid.toISOString(),
    Number.isNaN(new Date(0).setUTCFullYear(undefined)),
    utcMonthResult,
    utcMonth.toISOString(),
    utcMonthDateResult,
    utcMonthDate.toISOString(),
    utcMonthOverflow.toISOString(),
    Number.isNaN(utcMonthInvalidResult),
    Number.isNaN(utcMonthInvalid.getTime()),
    Number.isNaN(new Date(0).setUTCMonth(undefined, 1)),
    utcDateResult,
    utcDate.toISOString(),
    utcDateOverflowResult,
    utcDateOverflow.toISOString(),
    utcDateUnderflowResult,
    utcDateUnderflow.toISOString(),
    Number.isNaN(utcDateInvalidResult),
    Number.isNaN(utcDateInvalid.getTime()),
    Number.isNaN(new Date(0).setUTCDate(undefined)),
    utcHoursResult,
    utcHours.toISOString(),
    utcHoursPartsResult,
    utcHoursParts.toISOString(),
    utcMinutesResult,
    utcMinutes.toISOString(),
    utcSecondsResult,
    utcSeconds.toISOString(),
    utcMillisecondsResult,
    utcMilliseconds.toISOString(),
    Number.isNaN(new Date(NaN).setUTCHours(1, 2, 3, 4)),
    Number.isNaN(new Date(0).setUTCSeconds(undefined, 1)),
    Number.isNaN(utcNoArgsResult),
    Number.isNaN(utcNoArgs.getTime()),
    typeof new Date(0).getTimezoneOffset() === "number",
    Number.isNaN(new Date(NaN).getTimezoneOffset()),
    hasDateShape(new Date(0).toDateString()),
    hasDateShape(new Date("0020-01-01T00:00:00Z").toDateString()),
    hasTimeShape(new Date(0).toTimeString()),
    hasLocalStringShape(new Date(0).toString()),
    new Date(NaN).toString(),
    typeof Date() === "string",
    hasLocalStringShape(Date()),
    Number.isNaN(invalid.setTime(8640000000000001)),
    Number.isNaN(invalid.getTime()),
    Number.isNaN(new Date(8640000000000001).getTime()),
    Number.isNaN(new Date(Infinity).getTime()),
    new Date(97445006.9).getTime(),
    Object.is(new Date(-0).getTime(), 0),
    Number.isNaN(Date.UTC(275760, 8, 13, 0, 0, 0, 1)),
    Number.isNaN(new Date(275760, 8, 13, 0, 0, 0, 1).getTime()),
    new Date(NaN).toJSON(),
    JSON.stringify(new Date(NaN)),
    new Date(NaN).toUTCString(),
    Number.isNaN(new Date(NaN).getUTCFullYear())
  ].join("|");
})()
