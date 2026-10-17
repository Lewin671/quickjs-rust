(function() {
  var compact = new RegExp("^(Sun|Mon|Tue|Wed|Thu|Fri|Sat)(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[0-9]{2}[0-9]{4}[0-9]{2}:[0-9]{2}:[0-9]{2}GMT[+-][0-9]{4}(\\(.+\\))?$");
  var spaced = /^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$/;
  return [
    compact.test("MonJan01202000:00:00GMT+0000"),
    compact.test("Mon Jan 01 2020 00:00:00 GMT+0000"),
    spaced.test("Mon Jan 01 2020 00:00:00 GMT+0000"),
    spaced.test(new Date(0).toString())
  ].join(":");
})()
