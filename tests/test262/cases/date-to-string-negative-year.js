// Derived from: test/built-ins/Date/prototype/toString/negative-year.js
var negative1DigitYearToString = (new Date("-000001-07-01T00:00Z")).toString();
var negative2DigitYearToString = (new Date("-000012-07-01T00:00Z")).toString();
var negative3DigitYearToString = (new Date("-000123-07-01T00:00Z")).toString();
var negative4DigitYearToString = (new Date("-001234-07-01T00:00Z")).toString();
var negative5DigitYearToString = (new Date("-012345-07-01T00:00Z")).toString();
var negative6DigitYearToString = (new Date("-123456-07-01T00:00Z")).toString();

if (negative1DigitYearToString.split(" ")[3] !== "-0001") { throw "expected year -1 to serialize as -0001"; }
if (negative2DigitYearToString.split(" ")[3] !== "-0012") { throw "expected year -12 to serialize as -0012"; }
if (negative3DigitYearToString.split(" ")[3] !== "-0123") { throw "expected year -123 to serialize as -0123"; }
if (negative4DigitYearToString.split(" ")[3] !== "-1234") { throw "expected year -1234 to serialize as -1234"; }
if (negative5DigitYearToString.split(" ")[3] !== "-12345") { throw "expected year -12345 to serialize as -12345"; }
if (negative6DigitYearToString.split(" ")[3] !== "-123456") { throw "expected year -123456 to serialize as -123456"; }
