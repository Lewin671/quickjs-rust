// Derived from: test/built-ins/Map/prototype/forEach/iterates-values-added-after-foreach-begins.js
// Derived from: test/built-ins/Map/prototype/forEach/iterates-values-deleted-then-readded.js
// Derived from: test/built-ins/Set/prototype/forEach/iterates-values-added-after-foreach-begins.js
// Derived from: test/built-ins/Set/prototype/forEach/iterates-values-deleted-then-readded.js
var map = new Map([["foo", 0], ["bar", 1]]);
var seen = [];
map.forEach(function(value, key) {
  if (key === "foo") {
    map.set("baz", 2);
  }
  seen.push(key + ":" + value);
});
if (seen.join("|") !== "foo:0|bar:1|baz:2") {
  throw new Error("Map forEach should visit values added during iteration");
}

map = new Map([["foo", 0], ["bar", 1]]);
seen = [];
var count = 0;
map.forEach(function(value, key) {
  if (count === 0) {
    map.delete("foo");
    map.set("foo", "baz");
  }
  seen.push(key + ":" + value);
  count++;
});
if (seen.join("|") !== "foo:0|bar:1|foo:baz") {
  throw new Error("Map forEach should revisit deleted then re-added keys");
}

var set = new Set([1]);
seen = [];
set.forEach(function(value, entry, receiver) {
  if (value === 1) {
    set.add(2);
  }
  if (value === 2) {
    set.add(3);
  }
  if (entry !== value || receiver !== set) {
    throw new Error("Set forEach callback arguments are invalid");
  }
  seen.push(value);
});
if (seen.join("|") !== "1|2|3") {
  throw new Error("Set forEach should visit values added during iteration");
}

set = new Set(["foo", "bar"]);
seen = [];
count = 0;
set.forEach(function(value) {
  if (count === 0) {
    set.delete("foo");
    set.add("foo");
  }
  seen.push(value);
  count++;
});
if (seen.join("|") !== "foo|bar|foo") {
  throw new Error("Set forEach should revisit deleted then re-added values");
}
