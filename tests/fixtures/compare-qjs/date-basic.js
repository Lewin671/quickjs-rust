(function () {
  var value = new Date("1970-01-02T03:04:05.006Z");
  return [
    typeof Date,
    Date.length,
    Date.parse.length,
    Date.UTC.length,
    Date.prototype.getTime.length,
    value.getTime(),
    value.valueOf(),
    value.toISOString(),
    Date.UTC(1970, 0, 2, 3, 4, 5, 6),
    Date.parse("1970-01-02T03:04:05.006Z"),
    new Date(0).toISOString()
  ].join("|");
})()
