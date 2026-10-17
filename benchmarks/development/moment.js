// Development driver: moment (MIT) shifts and formats generated UTC instants.
// UTC mode keeps every result independent of the host time zone. Prints one line.
var N = 12000;

var hash = 0;
function mix(value) {
  hash = ((hash << 5) - hash + value) | 0;
}
var formats = ["YYYY-MM-DD", "HH:mm:ss.SSS", "ddd MMM D YYYY", "YYYY-MM-DDTHH:mm:ss[Z]", "MM/DD/YYYY h:mm a"];
var origin = moment.utc(0);

for (var i = 0; i < N; i++) {
  var instant = moment.utc(1700000000000)
    .add(i % 400, "days")
    .add(i % 24, "hours")
    .add(i % 60, "minutes")
    .add(i % 1000, "milliseconds");
  var text = instant.format(formats[i % formats.length]);
  mix(text.length);
  mix(text.charCodeAt(i % text.length));
  mix(instant.valueOf() % 100000);
  mix(instant.diff(origin, "hours"));
  mix(instant.clone().startOf("month").date());
  mix(instant.dayOfYear());
  mix(instant.month() + instant.date() + instant.hour());
  mix(instant.isBefore(origin.add(1, "year")) ? 1 : 0);
  mix(instant.utcOffset());
  mix(instant.toISOString().length);
  mix(instant.add(-3, "days").diff(instant, "days"));
  mix(instant.clone().endOf("day").hour());
}
print("CHECKSUM " + (hash >>> 0).toString(16));
