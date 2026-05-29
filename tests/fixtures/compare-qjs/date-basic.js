(function () {
  var value = new Date("1970-01-02T03:04:05.006Z");
  var custom = new Date("1970-01-02T03:04:05.006Z");
  custom.toISOString = function () { return "custom"; };
  return [
    typeof Date,
    Date.length,
    Date.parse.length,
    Date.UTC.length,
    Date.prototype.getTime.length,
    Date.prototype.getUTCFullYear.length,
    Date.prototype.toJSON.length,
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
    new Date(NaN).toJSON(),
    JSON.stringify(new Date(NaN)),
    new Date(NaN).toUTCString(),
    Number.isNaN(new Date(NaN).getUTCFullYear())
  ].join("|");
})()
