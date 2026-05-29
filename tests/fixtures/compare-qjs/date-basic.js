(function () {
  var value = new Date("1970-01-02T03:04:05.006Z");
  var custom = new Date("1970-01-02T03:04:05.006Z");
  var mutable = new Date(0);
  var setResult = mutable.setTime(97445006.9);
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
    new Date("0020-01-01T00:00:00Z").toUTCString(),
    JSON.stringify(value) === '"1970-01-02T03:04:05.006Z"',
    JSON.stringify({ when: value }) === '{"when":"1970-01-02T03:04:05.006Z"}',
    custom.toJSON(),
    JSON.stringify(custom) === '"custom"',
    setResult,
    mutable.getTime(),
    mutable.toISOString(),
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
