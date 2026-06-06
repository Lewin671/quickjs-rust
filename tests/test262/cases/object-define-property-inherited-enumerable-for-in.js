// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-404.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-414.js
// Derived from: test/built-ins/Object/defineProperty/15.2.3.6-4-419.js
Object.defineProperty(Boolean.prototype, "prop", {
  value: 1001,
  writable: true,
  enumerable: true,
  configurable: true,
});
let boolObj = new Boolean();
let verifyEnumerable = false;
for (let p in boolObj) {
  if (p === "prop") {
    verifyEnumerable = true;
  }
}
delete Boolean.prototype.prop;
if (boolObj.hasOwnProperty("prop") || !verifyEnumerable) {
  throw "expected inherited Boolean prototype property to enumerate";
}

let appointment = {};
Object.defineProperty(appointment, "startTime", {
  value: 1001,
  writable: true,
  enumerable: true,
  configurable: true,
});
Object.defineProperty(appointment, "name", {
  value: "NAME",
  writable: true,
  enumerable: true,
  configurable: true,
});
let meeting = Object.create(appointment);
Object.defineProperty(meeting, "conferenceCall", {
  value: "In-person meeting",
  writable: true,
  enumerable: true,
  configurable: true,
});
let teamMeeting = Object.create(meeting);
let seen = "";
for (let p in teamMeeting) {
  seen += p + "|";
}
if (
  teamMeeting.hasOwnProperty("name") ||
  teamMeeting.hasOwnProperty("startTime") ||
  teamMeeting.hasOwnProperty("conferenceCall") ||
  seen.indexOf("name|") < 0 ||
  seen.indexOf("startTime|") < 0 ||
  seen.indexOf("conferenceCall|") < 0
) {
  throw "expected inherited Object.create properties to enumerate";
}

let foo = function() {};
Object.defineProperty(Function.prototype, "prop", {
  value: 1001,
  writable: true,
  enumerable: true,
  configurable: true,
});
let bound = foo.bind({});
verifyEnumerable = false;
for (let p in bound) {
  if (p === "prop") {
    verifyEnumerable = true;
  }
}
delete Function.prototype.prop;
if (bound.hasOwnProperty("prop") || !verifyEnumerable) {
  throw "expected inherited Function prototype property to enumerate";
}
